use bastion::prelude::*;

#[macro_use]
extern crate log;

fn fib(n: usize) -> usize {
    if n == 0 || n == 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn deserialize_into_fib_command(message: &str) -> (String, usize) {
    let arguments: Vec<&str> = message.split(' ').collect();
    let command = arguments
        .first()
        .map(|s| s.to_string())
        .unwrap_or(String::new());
    let number = usize::from_str_radix(arguments.get(1).unwrap_or(&"0"), 10).unwrap_or(0);
    (command, number)
}

async fn fib_child_task(ctx: BastionContext) -> Result<(), ()> {
    loop {
        msg! { ctx.recv().await?,
            question: &'static str =!> {
                let (command, number) = deserialize_into_fib_command(question);
                if command == "fib" {
                    answer!(ctx, format!("the answer is {}!", fib(number))).expect("couldn't reply :(");
                } else {
                    answer!(ctx, "I'm sorry I didn't understand the task I was supposed to do").expect("couldn't reply :(");
                }
            };
            ref broadcast: &'static str => {
                warn!("{} received broadcast\n{:?}", ctx.signature().path(), *broadcast);
            };
            whisper: &'static str => {
                info!("{} someone told me something\n{}", ctx.signature().path(), whisper);
            };
            unknown:_ => {
                error!("uh oh, I received a message I didn't understand\n {:?}", unknown);
            };
        }
    }
}

async fn send_command_to_child(child: &ChildRef, command: &'static str) -> std::io::Result<String> {
    let answer = child
        .ask_anonymously(command)
        .expect("Couldn't ask question to child.");

    let response = msg! { answer.await.expect("Couldn't receive the answer."),
        msg: &'static str => {
            msg
        };
        _: _ => "";
    };

    dbg!(response);

    Ok(response.to_string())
}
fn main() {
    env_logger::init();

    Bastion::init();
    Bastion::start();

    let children = Bastion::children(|children| {
        children
            // ...containing a defined number of elements...
            .with_redundancy(4)
            .with_exec(fib_child_task)
    })
    .expect("couldn't create children");

    children
        .broadcast("Hey everyone I hope you're ready to do some really cool stuff!")
        .expect("Couldn't broadcast to the children.");

    for child in children.elems() {
        child
            .tell_anonymously("shhh here's a message, don't tell anyone.")
            .expect("Couldn't whisper to child.");

        let reply = run!(send_command_to_child(child, "Hey there, what's up?"))
            .expect("send_command_to_child failed");
        println!("got response: {}", reply);

        let fib_reply =
            run!(send_command_to_child(child, "fib 25")).expect("send_command_to_child failed");
        println!("got response: {}", fib_reply);
    }

    Bastion::stop();
    Bastion::block_until_stopped();
}
