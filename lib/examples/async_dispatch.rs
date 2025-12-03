//! Watch for messages from the zwlr_output_manager_v1 protocol.
//!
//! May be used to check if the display configuration has changed.

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = cosmic_randr::channel();

    tokio::spawn(async move {
        let Ok((mut context, mut event_queue)) = cosmic_randr::connect(tx) else {
            return;
        };

        loop {
            if dbg!(context.dispatch(&mut event_queue).await).is_err() {
                return;
            }
        }
    });

    while let Some(event) = rx.recv().await {
        eprintln!("{event:?}");
    }

    Ok(())
}
