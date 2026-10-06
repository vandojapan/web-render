use std::{sync::Arc, time::Duration};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::sync::mpsc::UnboundedSender;

struct ParentProbe {
    system: System,
    pid: Pid,
    start_time: Option<u64>,
}

impl ParentProbe {
    fn new(pid: u32) -> Self {
        Self {
            system: System::new(),
            pid: Pid::from_u32(pid),
            start_time: None,
        }
    }

    fn is_alive(&mut self) -> bool {
        // No CPU, memory, environment or command-line refresh for unrelated processes.
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self.pid]),
            true,
            ProcessRefreshKind::nothing(),
        );
        let Some(process) = self.system.process(self.pid) else {
            return false;
        };
        let start_time = process.start_time();
        let original = self.start_time.get_or_insert(start_time);
        *original == start_time
    }
}

pub async fn watch(parent_pid: u32, sender: Arc<UnboundedSender<()>>) {
    watch_interval(parent_pid, sender, Duration::from_secs(5)).await;
}

async fn watch_interval(parent_pid: u32, sender: Arc<UnboundedSender<()>>, interval: Duration) {
    let mut probe = ParentProbe::new(parent_pid);
    loop {
        if sender.is_closed() {
            return;
        }
        // Even a targeted OS query can block. Never run it on CEF's current-thread runtime.
        let check = tokio::task::spawn_blocking(move || {
            let started = std::time::Instant::now();
            let alive = probe.is_alive();
            tracing::debug!(
                elapsed_ms = started.elapsed().as_secs_f64() * 1000.0,
                "Parent process check finished"
            );
            (probe, alive)
        });
        let result = tokio::select! {
            result = check => result,
            _ = sender.closed() => return,
        };
        match result {
            Ok((returned_probe, true)) => probe = returned_probe,
            Ok((_, false)) => {
                tracing::info!(parent_pid, "Parent process has exited, shutting down");
                let _ = sender.send(());
                return;
            }
            Err(error) => {
                tracing::error!(%error, "Parent process monitor failed, shutting down");
                let _ = sender.send(());
                return;
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(interval) => {},
            _ = sender.closed() => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "child fixture invoked by the parent-watch test"]
    fn child_waits() {
        std::thread::sleep(Duration::from_secs(60));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn detects_parent_exit_and_stops_when_receiver_closes() {
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "parent_process::tests::child_waits", "--ignored"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let mut child = command.spawn().unwrap();
        let child_pid = child.id().unwrap();
        assert!(ParentProbe::new(child_pid).is_alive());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(watch_interval(
            child_pid,
            Arc::new(tx),
            Duration::from_millis(10),
        ));
        assert!(
            tokio::time::timeout(Duration::from_millis(50), rx.recv())
                .await
                .is_err()
        );
        child.kill().await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), rx.recv())
                .await
                .unwrap(),
            Some(())
        );
        task.await.unwrap();

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(watch(std::process::id(), Arc::new(tx)));
        tokio::time::sleep(Duration::from_millis(20)).await;
        drop(rx);
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap();
    }
}
