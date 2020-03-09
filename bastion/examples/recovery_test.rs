use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bastion::prelude::*;
use futures_timer::Delay;

fn main() {
    Bastion::init();

    Bastion::supervisor(supervisor).expect("Couldn't create the supervisor.");

    Bastion::start();
    Bastion::block_until_stopped();
}

struct RoundRobinHandler {
    index: AtomicU64,
}

impl RoundRobinHandler {
    pub fn new() -> Self {
        RoundRobinHandler {
            index: AtomicU64::new(0),
        }
    }
}

impl DispatcherHandler for RoundRobinHandler {
    fn notify(
        &self,
        _from_child: &ChildRef,
        _entries: &DispatcherMap,
        _notification_type: NotificationType,
    ) {
    }

    fn broadcast_message(&self, entries: &DispatcherMap, message: &Arc<SignedMessage>) {
        let current_index = self.index.load(Ordering::SeqCst) % entries.len() as u64;

        let mut skipped = 0;
        for pair in entries.iter() {
            if skipped != current_index {
                skipped += 1;
                continue;
            }

            let entry = pair.key();
            match entry.tell_anonymously(message.clone()) {
                Ok(_) => println!("OK"),
                Err(_) => println!("NOT OK"),
            }
            break;
        }

        self.index.store(current_index + 1, Ordering::SeqCst);
    }
}


fn supervisor(supervisor: Supervisor) -> Supervisor {
    let restart_strategy = RestartStrategy::default()
        .with_restart_policy(RestartPolicy::Tries(3))
        .with_actor_restart_strategy(ActorRestartStrategy::LinearBackOff {
            timeout: Duration::from_secs(1),
        });

    supervisor
        .with_restart_strategy(restart_strategy)
        .children(|children| failed_actors_group(children))
}

fn failed_actors_group(children: Children) -> Children {
    children
        .with_dispatcher(
            Dispatcher::default()
                .with_dispatcher_type(DispatcherType::Named("Test".to_string()))
                .with_handler(Box::new(RoundRobinHandler::new())),
        )
        .with_exec(move |ctx: BastionContext| async move {
            println!("Worker started!");

            let data = vec!["A B C", "A C C", "B C C"];
            let group_name = "Test".to_string();
            let target = BroadcastTarget::Group(group_name);

            for input in data {
                ctx.broadcast_message(target.clone(), input);
            }

            Delay::new(Duration::from_secs(1)).await;

            panic!("Unexpected error...");
    })
}
