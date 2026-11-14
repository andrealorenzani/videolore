// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Process scheduler: never more than [`MAX_PROCESSES`] ffmpeg/ffprobe processes at once.
//!
//! Every process is started through a [`Limited`] runner, which takes slots from the shared
//! [`Scheduler`] before spawning and gives them back when the process ends (wait, kill or
//! drop). Waiters are served by strict priority (preview > export > probe > background) and
//! in arrival order inside a level. A waiting preview or export may preempt a running
//! preemptible job (proxy generation), which kills its ffmpeg and queues again.
//! [`Pool`] is a small fixed-size worker pool used for probes and proxies.

use crate::ffmpeg::{CmdOutput, Process, Runner};
use std::ffi::OsString;
use std::io::{self, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

/// The default process cap of the whole application ("How hard videolore works" =
/// Light or Balanced).
pub const MAX_PROCESSES: u8 = 3;
/// The highest cap any level allows (Maximum on big machines).
pub const MAX_CAP: u8 = 6;

/// How often a waiter re-checks its cancel flag.
const POLL: Duration = Duration::from_millis(20);

/// Who is asking for a process. Lower value = served first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Preview = 0,
    Export = 1,
    Probe = 2,
    Background = 3,
}

#[derive(Debug)]
struct Waiter {
    ticket: u64,
    priority: Priority,
}

#[derive(Debug)]
struct Running {
    ticket: u64,
    priority: Priority,
    preempt: Option<Arc<AtomicBool>>,
}

#[derive(Debug, Default)]
struct State {
    cap: u8,
    in_use: u8,
    peak: u8,
    next: u64,
    waiting: Vec<Waiter>,
    running: Vec<Running>,
    granted: u64,
}

/// Shared slot counter with a priority queue.
#[derive(Debug)]
pub struct Scheduler {
    state: Mutex<State>,
    cv: Condvar,
}

impl Scheduler {
    pub fn new(cap: u8) -> Arc<Scheduler> {
        Arc::new(Scheduler {
            state: Mutex::new(State {
                cap: cap.clamp(1, MAX_CAP),
                ..State::default()
            }),
            cv: Condvar::new(),
        })
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn cap(&self) -> u8 {
        self.lock().cap
    }

    /// Changes the cap (1..=[`MAX_CAP`]). Running processes are never stopped: when the cap
    /// goes down, new requests simply wait until enough slots are free.
    pub fn set_cap(&self, cap: u8) {
        self.lock().cap = cap.clamp(1, MAX_CAP);
        self.cv.notify_all();
    }

    /// Slots currently held.
    pub fn in_use(&self) -> u8 {
        self.lock().in_use
    }

    /// Highest number of slots ever held at once.
    pub fn peak(&self) -> u8 {
        self.lock().peak
    }

    /// Requests waiting for a slot.
    pub fn waiting(&self) -> usize {
        self.lock().waiting.len()
    }

    /// Total number of permits handed out so far.
    pub fn granted(&self) -> u64 {
        self.lock().granted
    }

    /// Waits for `slots` slots. Returns `None` when `cancel` is set while waiting.
    /// `preempt` is set by the scheduler when a more urgent job needs this job's slots.
    pub fn acquire(
        self: &Arc<Self>,
        priority: Priority,
        slots: u8,
        cancel: Option<&AtomicBool>,
        preempt: Option<Arc<AtomicBool>>,
    ) -> Option<Permit> {
        let mut st = self.lock();
        let ticket = st.next;
        st.next += 1;
        st.waiting.push(Waiter { ticket, priority });
        loop {
            if cancel.is_some_and(|c| c.load(Ordering::SeqCst)) {
                st.waiting.retain(|w| w.ticket != ticket);
                drop(st);
                self.cv.notify_all();
                return None;
            }
            let first = !st
                .waiting
                .iter()
                .any(|w| (w.priority, w.ticket) < (priority, ticket));
            let slots = slots.clamp(1, st.cap);
            if first && st.in_use + slots <= st.cap {
                st.waiting.retain(|w| w.ticket != ticket);
                st.in_use += slots;
                st.peak = st.peak.max(st.in_use);
                st.granted += 1;
                st.running.push(Running {
                    ticket,
                    priority,
                    preempt,
                });
                return Some(Permit {
                    sched: Arc::clone(self),
                    ticket,
                    slots,
                });
            }
            if first && priority <= Priority::Export {
                Self::request_preemption(&st, priority);
            }
            st = self
                .cv
                .wait_timeout(st, POLL)
                .map(|(g, _)| g)
                .unwrap_or_else(|e| e.into_inner().0);
        }
    }

    /// Asks the least urgent preemptible job (less urgent than `priority`) to stop.
    fn request_preemption(st: &State, priority: Priority) {
        let victim = st
            .running
            .iter()
            .filter(|r| r.priority > priority)
            .filter_map(|r| r.preempt.as_ref().map(|p| (r.priority, r.ticket, p)))
            .filter(|(_, _, p)| !p.load(Ordering::SeqCst))
            .max_by_key(|(prio, ticket, _)| (*prio, *ticket));
        if let Some((_, _, flag)) = victim {
            flag.store(true, Ordering::SeqCst);
        }
    }

    fn release(&self, ticket: u64, slots: u8) {
        let mut st = self.lock();
        st.in_use = st.in_use.saturating_sub(slots);
        st.running.retain(|r| r.ticket != ticket);
        drop(st);
        self.cv.notify_all();
    }

    /// A runner whose processes count against this scheduler at `priority`.
    pub fn runner(self: &Arc<Self>, priority: Priority, inner: Arc<dyn Runner>) -> Limited {
        Limited {
            sched: Arc::clone(self),
            inner,
            priority,
            cancel: None,
            preempt: None,
        }
    }
}

/// Slots held; released on drop.
#[derive(Debug)]
pub struct Permit {
    sched: Arc<Scheduler>,
    ticket: u64,
    slots: u8,
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.sched.release(self.ticket, self.slots);
    }
}

/// A [`Runner`] that holds scheduler slots for the lifetime of each process.
#[derive(Clone)]
pub struct Limited {
    sched: Arc<Scheduler>,
    inner: Arc<dyn Runner>,
    priority: Priority,
    cancel: Option<Arc<AtomicBool>>,
    preempt: Option<Arc<AtomicBool>>,
}

impl std::fmt::Debug for Limited {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Limited")
            .field("priority", &self.priority)
            .finish()
    }
}

/// Error returned when a cancelled job stops waiting for a slot.
pub fn cancelled() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "cancelled")
}

impl Limited {
    /// Same runner; waiting for a slot stops when `flag` is set.
    pub fn cancellable(&self, flag: Arc<AtomicBool>) -> Limited {
        Limited {
            cancel: Some(flag),
            ..self.clone()
        }
    }

    /// Same runner; the scheduler sets `flag` when a more urgent job needs the slot.
    pub fn preemptible(&self, flag: Arc<AtomicBool>) -> Limited {
        Limited {
            preempt: Some(flag),
            ..self.clone()
        }
    }

    pub fn priority(&self) -> Priority {
        self.priority
    }

    pub fn scheduler(&self) -> &Arc<Scheduler> {
        &self.sched
    }

    fn permit(&self, slots: u8) -> io::Result<Arc<Permit>> {
        self.sched
            .acquire(
                self.priority,
                slots,
                self.cancel.as_deref(),
                self.preempt.clone(),
            )
            .map(Arc::new)
            .ok_or_else(cancelled)
    }
}

impl Runner for Limited {
    fn run(&self, program: &Path, args: &[OsString]) -> io::Result<CmdOutput> {
        let _permit = self.permit(1)?;
        self.inner.run(program, args)
    }

    fn run_within(
        &self,
        program: &Path,
        args: &[OsString],
        timeout: Duration,
    ) -> io::Result<CmdOutput> {
        let _permit = self.permit(1)?;
        self.inner.run_within(program, args, timeout)
    }

    fn spawn_in(
        &self,
        program: &Path,
        args: &[OsString],
        cwd: Option<&Path>,
    ) -> io::Result<Box<dyn Process>> {
        let permit = self.permit(1)?;
        let inner = self.inner.spawn_in(program, args, cwd)?;
        Ok(Box::new(Held {
            inner,
            permit: Some(permit),
        }))
    }

    fn spawn_pair(
        &self,
        a: (&Path, &[OsString]),
        b: (&Path, &[OsString]),
        cwd: Option<&Path>,
    ) -> io::Result<(Box<dyn Process>, Box<dyn Process>)> {
        let permit = self.permit(2)?;
        let first = self.inner.spawn_in(a.0, a.1, cwd)?;
        let second = self.inner.spawn_in(b.0, b.1, cwd)?;
        Ok((
            Box::new(Held {
                inner: first,
                permit: Some(Arc::clone(&permit)),
            }),
            Box::new(Held {
                inner: second,
                permit: Some(permit),
            }),
        ))
    }
}

/// A process plus the slots it holds.
struct Held {
    inner: Box<dyn Process>,
    permit: Option<Arc<Permit>>,
}

impl Process for Held {
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
        self.inner.take_stdout()
    }

    fn kill(&mut self) {
        self.inner.kill();
        let _ = self.inner.wait();
        self.permit = None;
    }

    fn wait(&mut self) -> io::Result<CmdOutput> {
        let r = self.inner.wait();
        self.permit = None;
        r
    }
}

/// Fixed number of worker threads processing jobs in submission order.
pub struct Pool<T: Send + 'static, R: Send + 'static> {
    tx: Option<Sender<T>>,
    rx: Receiver<R>,
}

impl<T: Send + 'static, R: Send + 'static> Pool<T, R> {
    /// Starts `n` workers running `f` on each job; `None` results are not reported.
    pub fn new(n: usize, f: impl Fn(T) -> Option<R> + Send + Sync + 'static) -> Pool<T, R> {
        let (tx, jobs) = channel::<T>();
        let (out, rx) = channel::<R>();
        let jobs = Arc::new(Mutex::new(jobs));
        let f = Arc::new(f);
        for _ in 0..n.max(1) {
            let jobs = Arc::clone(&jobs);
            let out = out.clone();
            let f = Arc::clone(&f);
            std::thread::spawn(move || {
                loop {
                    let job = match jobs.lock() {
                        Ok(rx) => rx.recv(),
                        Err(_) => return,
                    };
                    let Ok(job) = job else { return };
                    if let Some(r) = f(job)
                        && out.send(r).is_err()
                    {
                        return;
                    }
                }
            });
        }
        Pool { tx: Some(tx), rx }
    }

    pub fn submit(&self, job: T) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(job);
        }
    }

    /// Results available now.
    pub fn results(&self) -> Vec<R> {
        self.rx.try_iter().collect()
    }

    /// Waits up to `timeout` for one result (tests).
    pub fn recv_timeout(&self, timeout: Duration) -> Option<R> {
        self.rx.recv_timeout(timeout).ok()
    }
}

impl<T: Send + 'static, R: Send + 'static> Drop for Pool<T, R> {
    fn drop(&mut self) {
        // Workers end after their current job once the queue is closed.
        self.tx = None;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ffmpeg::fake::{FakeRunner, ok};
    use std::sync::atomic::AtomicUsize;
    use std::time::Instant;

    /// A runner that counts live processes and keeps each one alive for `delay`.
    pub struct CountingRunner {
        pub live: Arc<AtomicUsize>,
        pub peak: Arc<AtomicUsize>,
        pub delay: Duration,
    }

    impl CountingRunner {
        pub fn new(delay: Duration) -> CountingRunner {
            CountingRunner {
                live: Arc::new(AtomicUsize::new(0)),
                peak: Arc::new(AtomicUsize::new(0)),
                delay,
            }
        }

        fn enter(&self) {
            let n = self.live.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(n, Ordering::SeqCst);
        }
    }

    struct CountedProcess {
        live: Arc<AtomicUsize>,
        done: bool,
        delay: Duration,
    }

    impl CountedProcess {
        fn finish(&mut self) {
            if !self.done {
                self.done = true;
                self.live.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }

    impl Process for CountedProcess {
        fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
            Some(Box::new(io::Cursor::new(b"progress=end\n".to_vec())))
        }
        fn kill(&mut self) {
            self.finish();
        }
        fn wait(&mut self) -> io::Result<CmdOutput> {
            if !self.done {
                std::thread::sleep(self.delay);
            }
            self.finish();
            Ok(CmdOutput {
                success: true,
                ..Default::default()
            })
        }
    }

    impl Drop for CountedProcess {
        fn drop(&mut self) {
            self.finish();
        }
    }

    impl Runner for CountingRunner {
        fn run(&self, _p: &Path, _a: &[OsString]) -> io::Result<CmdOutput> {
            self.enter();
            std::thread::sleep(self.delay);
            self.live.fetch_sub(1, Ordering::SeqCst);
            Ok(CmdOutput {
                success: true,
                stdout: crate::media::tests::MP4_JSON.as_bytes().to_vec(),
                stderr: String::new(),
            })
        }
        fn spawn_in(
            &self,
            _p: &Path,
            _a: &[OsString],
            _c: Option<&Path>,
        ) -> io::Result<Box<dyn Process>> {
            self.enter();
            Ok(Box::new(CountedProcess {
                live: Arc::clone(&self.live),
                done: false,
                delay: self.delay,
            }))
        }
    }

    /// 20 probes + an export + preview playback and stills through `sched`; returns the
    /// highest number of processes alive at once.
    fn busy_workload(sched: &Arc<Scheduler>) -> usize {
        let counting = Arc::new(CountingRunner::new(Duration::from_millis(10)));
        let mut threads = Vec::new();
        for _ in 0..20 {
            let r = sched.runner(Priority::Probe, counting.clone());
            threads.push(std::thread::spawn(move || {
                r.run(Path::new("ffprobe"), &[]).unwrap();
            }));
        }
        let r = sched.runner(Priority::Export, counting.clone());
        threads.push(std::thread::spawn(move || {
            let mut p = r.spawn(Path::new("ffmpeg"), &[]).unwrap();
            p.wait().unwrap();
        }));
        let r = sched.runner(Priority::Preview, counting.clone());
        threads.push(std::thread::spawn(move || {
            for _ in 0..3 {
                let (mut a, mut b) = r
                    .spawn_pair((Path::new("f"), &[]), (Path::new("f"), &[]), None)
                    .unwrap();
                a.wait().unwrap();
                b.kill();
            }
        }));
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(counting.live.load(Ordering::SeqCst), 0, "all jobs complete");
        counting.peak.load(Ordering::SeqCst)
    }

    // covers: REQ-033, REQ-050
    #[test]
    fn every_level_keeps_its_process_limit_and_all_jobs_complete() {
        let fp = |l| crate::machine::Machine::simulated(l, l / 2, 16 << 30).fingerprint();
        let mut caps = Vec::new();
        for l in [1, 8, 16] {
            for e in crate::settings::Effort::ALL {
                caps.push(crate::settings::budget(e, &fp(l)).max_processes);
            }
        }
        caps.sort();
        caps.dedup();
        assert_eq!(caps, vec![3, 4, 6]);
        let runs: Vec<_> = caps
            .into_iter()
            .map(|cap| {
                std::thread::spawn(move || {
                    let sched = Scheduler::new(cap);
                    let peak = busy_workload(&sched);
                    (cap, peak, sched.peak(), sched.in_use(), sched.waiting())
                })
            })
            .collect();
        for r in runs {
            let (cap, peak, sched_peak, in_use, waiting) = r.join().unwrap();
            assert!(peak <= cap as usize, "cap {cap}: {peak} processes");
            assert!(sched_peak <= cap);
            assert_eq!((in_use, waiting), (0, 0));
        }
        assert_eq!(Scheduler::new(99).cap(), MAX_CAP);
    }

    // covers: REQ-050
    #[test]
    fn a_lower_cap_waits_for_running_processes_instead_of_stopping_them() {
        let sched = Scheduler::new(6);
        let held: Vec<Permit> = (0..5)
            .map(|_| sched.acquire(Priority::Export, 1, None, None).unwrap())
            .collect();
        sched.set_cap(3);
        assert_eq!(sched.cap(), 3);
        assert_eq!(sched.in_use(), 5, "running processes are not stopped");
        let s = Arc::clone(&sched);
        let t = std::thread::spawn(move || s.acquire(Priority::Preview, 1, None, None).is_some());
        let mut held = held;
        // still 4 of 3 after one ends: the new request keeps waiting
        held.pop();
        std::thread::sleep(Duration::from_millis(30));
        assert!(!t.is_finished());
        held.truncate(2);
        assert!(t.join().unwrap());
        // raising the cap lets more run at once
        sched.set_cap(6);
        let more: Vec<Permit> = (0..4)
            .map(|_| sched.acquire(Priority::Probe, 1, None, None).unwrap())
            .collect();
        assert_eq!(sched.in_use(), 6);
        drop(more);
        drop(held);
        assert_eq!(sched.in_use(), 0);
    }

    // covers: REQ-033
    #[test]
    fn never_more_than_three_processes_and_all_jobs_complete() {
        let counting = Arc::new(CountingRunner::new(Duration::from_millis(15)));
        let sched = Scheduler::new(MAX_PROCESSES);
        let mut threads = Vec::new();
        // 20 probes
        for _ in 0..20 {
            let r = sched.runner(Priority::Probe, counting.clone());
            threads.push(std::thread::spawn(move || {
                r.run(Path::new("ffprobe"), &[]).unwrap();
            }));
        }
        // one export
        let r = sched.runner(Priority::Export, counting.clone());
        threads.push(std::thread::spawn(move || {
            let mut p = r.spawn(Path::new("ffmpeg"), &[]).unwrap();
            p.wait().unwrap();
        }));
        // preview playback (a pair) and a few still frames
        let r = sched.runner(Priority::Preview, counting.clone());
        threads.push(std::thread::spawn(move || {
            for _ in 0..3 {
                let (mut a, mut b) = r
                    .spawn_pair((Path::new("f"), &[]), (Path::new("f"), &[]), None)
                    .unwrap();
                a.wait().unwrap();
                b.kill();
                let mut s = r.spawn(Path::new("f"), &[]).unwrap();
                drop(s.take_stdout());
                drop(s);
            }
        }));
        for t in threads {
            t.join().unwrap();
        }
        assert!(counting.peak.load(Ordering::SeqCst) <= 3);
        assert!(sched.peak() <= 3);
        assert_eq!(sched.in_use(), 0);
        assert_eq!(sched.waiting(), 0);
        assert_eq!(sched.granted(), 20 + 1 + 6);
        assert_eq!(counting.live.load(Ordering::SeqCst), 0);
    }

    // covers: REQ-033
    #[test]
    fn strict_priority_then_arrival_order() {
        let sched = Scheduler::new(1);
        let hold = sched.acquire(Priority::Export, 1, None, None).unwrap();
        let order = Arc::new(Mutex::new(Vec::new()));
        let mut threads = Vec::new();
        for (i, p) in [
            Priority::Background,
            Priority::Probe,
            Priority::Probe,
            Priority::Preview,
        ]
        .into_iter()
        .enumerate()
        {
            let s = Arc::clone(&sched);
            let o = Arc::clone(&order);
            threads.push(std::thread::spawn(move || {
                let _p = s.acquire(p, 1, None, None).unwrap();
                o.lock().unwrap().push(i);
            }));
            // make arrival order deterministic
            let deadline = Instant::now() + Duration::from_secs(5);
            while sched.waiting() < i + 1 && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        drop(hold);
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(*order.lock().unwrap(), vec![3, 1, 2, 0]);
    }

    // covers: REQ-033
    #[test]
    fn cancel_while_waiting_and_preemption_of_background_work() {
        let sched = Scheduler::new(1);
        let preempt = Arc::new(AtomicBool::new(false));
        let proxy = sched
            .acquire(Priority::Background, 1, None, Some(Arc::clone(&preempt)))
            .unwrap();
        // a cancelled probe gives up waiting
        let cancel = Arc::new(AtomicBool::new(false));
        let s = Arc::clone(&sched);
        let c = Arc::clone(&cancel);
        let t = std::thread::spawn(move || s.acquire(Priority::Probe, 1, Some(&c), None).is_none());
        std::thread::sleep(Duration::from_millis(30));
        assert!(!preempt.load(Ordering::SeqCst), "probes do not preempt");
        cancel.store(true, Ordering::SeqCst);
        assert!(t.join().unwrap());
        // a preview request preempts the proxy job, which then releases its slot
        let s = Arc::clone(&sched);
        let t = std::thread::spawn(move || s.acquire(Priority::Preview, 1, None, None).is_some());
        let deadline = Instant::now() + Duration::from_secs(5);
        while !preempt.load(Ordering::SeqCst) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(preempt.load(Ordering::SeqCst));
        drop(proxy); // the proxy job reacts by killing its ffmpeg
        assert!(t.join().unwrap());
        assert_eq!(sched.in_use(), 0);
        assert_eq!(sched.cap(), 1);
    }

    #[test]
    fn limited_runner_holds_slots_until_the_process_ends() {
        let sched = Scheduler::new(3);
        let fake: Arc<dyn Runner> = Arc::new(FakeRunner::default().on(|_, _| ok("x")));
        let r = sched.runner(Priority::Export, fake);
        assert_eq!(r.priority(), Priority::Export);
        assert!(format!("{r:?}").contains("Export"));
        let mut p = r.spawn(Path::new("ffmpeg"), &[]).unwrap();
        assert_eq!(r.scheduler().in_use(), 1);
        let mut s = String::new();
        p.take_stdout().unwrap().read_to_string(&mut s).unwrap();
        assert_eq!(s, "x");
        p.wait().unwrap();
        assert_eq!(sched.in_use(), 0);
        let (a, b) = r
            .spawn_pair((Path::new("a"), &[]), (Path::new("b"), &[]), None)
            .unwrap();
        assert_eq!(sched.in_use(), 2);
        drop(a);
        assert_eq!(sched.in_use(), 2, "the pair shares its slots");
        drop(b);
        assert_eq!(sched.in_use(), 0);
        assert_eq!(r.run(Path::new("x"), &[]).unwrap().stdout, b"x");
        // a cancelled runner never starts anything
        let flag = Arc::new(AtomicBool::new(true));
        let hold = sched.acquire(Priority::Preview, 3, None, None).unwrap();
        let rc = r.cancellable(flag);
        assert_eq!(
            rc.run(Path::new("x"), &[]).unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        assert!(rc.spawn(Path::new("x"), &[]).is_err());
        assert!(
            rc.spawn_pair((Path::new("a"), &[]), (Path::new("b"), &[]), None)
                .is_err()
        );
        drop(hold);
        let pre = r.preemptible(Arc::new(AtomicBool::new(false)));
        let mut p = pre.spawn(Path::new("x"), &[]).unwrap();
        p.kill();
        assert_eq!(sched.in_use(), 0);
    }

    #[test]
    fn pool_runs_jobs_on_workers() {
        let pool: Pool<u32, u32> = Pool::new(3, |n| (n % 2 == 0).then_some(n * 10));
        for n in 0..6 {
            pool.submit(n);
        }
        let mut got = Vec::new();
        while got.len() < 3 {
            got.push(pool.recv_timeout(Duration::from_secs(5)).unwrap());
        }
        got.sort();
        assert_eq!(got, vec![0, 20, 40]);
        assert!(pool.results().is_empty());
    }
}
