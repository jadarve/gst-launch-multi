mod cli;
mod pipeline;

use anyhow::Result;
use clap::Parser;
use gst::glib;
use gst::prelude::ClockExt;

use pipeline::run_pipeline;
use tokio::sync::{broadcast, OnceCell};

// It is important that all pipelines share both the same clock and basetime.
// Using this OnceCell, we guarantee that the instances are created only once.
static GST_CLOCK_AND_BASETIME: OnceCell<(gst::Clock, gst::ClockTime)> = OnceCell::const_new();

#[tokio::main]
async fn main() -> Result<()> {
    gst::init()?;

    let cli_args = cli::CliArgs::parse()?;

    // create shutdown channel
    let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

    // The command channel is used by the CLI task to broadcast commands to each pipelines
    // and the pipeline_controller_task
    let (command_tx, command_rx) = broadcast::channel::<cli::SubCommand>(1);

    // There are two main tasks:
    //- The pipeline_controller_task is responsible for holding all the pipelines tasks.
    //- The cli_task is responsible for handling the CLI commands once the application starts.
    let mut task_set = tokio::task::JoinSet::new();
    task_set.spawn_blocking(move || pipeline_controller_task(cli_args, shutdown_rx, command_rx));
    task_set.spawn_blocking(move || cli_task(shutdown_tx, command_tx));

    // wait for all the tasks to complete
    while task_set.join_next().await.is_some() {}

    Ok(())
}

fn pipeline_controller_task(
    cli_args: cli::CliArgs,
    shutdown_rx: broadcast::Receiver<()>,
    mut command_rx: broadcast::Receiver<cli::SubCommand>,
) -> Result<()> {
    // as single GLib MainLoop for all pipelines
    let main_loop = glib::MainLoop::new(None, false);

    // Stores all the tasks handlers for running pipelines.
    let mut pipeline_handlers = tokio::task::JoinSet::new();

    // create a new task for handling CLI commands
    // and stopping GLib main loop when all pipelines are finished
    let main_loop_clone = main_loop.clone();
    tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(async move {
            // get ownership of the main loop
            let main_loop = main_loop_clone;

            // first, create the pipeline handlers for the pipelines described in cli_args
            for pipeline_config in cli_args.pipeline_config {
                let rx = shutdown_rx.resubscribe();
                let cmd_rx = command_rx.resubscribe();

                pipeline_handlers.spawn_blocking(move || {
                    let _ = create_and_run_pipeline(pipeline_config.clone(), rx, cmd_rx);
                });

                // NOTE: this sleep is needed to give pipelines time to start, removing it
                // causes some segmentation fault.
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }

            // Handle CLI commands
            while let Ok(command) = command_rx.recv().await {
                // this task only handles the AddPipeline command, all other commands
                // are handle internally in each pipeline task
                if let cli::SubCommand::AddPipeline(args) = command {
                    let pipeline_shutdown_rx = shutdown_rx.resubscribe();
                    let cmd_rx = command_rx.resubscribe();

                    let args_clone = args.clone();
                    pipeline_handlers.spawn_blocking(move || {
                        let _ = create_and_run_pipeline(args_clone, pipeline_shutdown_rx, cmd_rx);
                    });
                }
            }

            // wait for all pipelines to finish
            while pipeline_handlers.join_next().await.is_some() {}

            // then stop the main loop
            println!("All pipelines finished, stopping main loop");
            main_loop.quit();
        })
    });

    // The function blocks here until the main loop is stopped in the task above
    main_loop.run();
    Ok(())
}

fn create_and_run_pipeline(
    config: cli::PipelineConfig,
    shutdown_rx: broadcast::Receiver<()>,
    command_rx: broadcast::Receiver<cli::SubCommand>,
) -> Result<()> {
    tokio::runtime::Handle::current().block_on(async move {
        // Gets a reference of the global clock and basetime. This pattern guarantees that the
        // basetime, in particular, is obtained only once during the lifetime of the application.
        // Every pipeline created by the application must share the same clock and basetime.
        let (clock, basetime) = GST_CLOCK_AND_BASETIME
            .get_or_init(|| async {
                let clock = gst::SystemClock::obtain();
                let basetime = clock.time().unwrap();
                (clock, basetime)
            })
            .await;

        let pipeline = pipeline::Pipeline::new(&config, Some(clock), basetime).unwrap();
        let pipeline_name = pipeline.config.name.clone();

        println!("Pipeline: {}: starting", pipeline_name);
        match pipeline.run(shutdown_rx, command_rx).await {
            Ok(_) => {}
            Err(err) => {
                println!("Pipeline: {}: task failed: {}", pipeline_name, err);
            }
        }
    });

    Ok(())
}

fn cli_task(
    shutdown_tx: broadcast::Sender<()>,
    command_tx: broadcast::Sender<cli::SubCommand>,
) -> Result<()> {
    loop {
        println!("\n\nEnter command (or help):");
        // need to start the string with the "binary" name
        let mut command: String = "cli ".to_string();

        let input: String = text_io::read!("{}\n");
        command.push_str(input.as_str());

        // try parsing the command from user input
        match cli::Cli::try_parse_from(command.split_whitespace()) {
            Ok(cli_command) => {
                // then broadcast the command to all pipelines
                match command_tx.send(cli_command.sub_command.clone()) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Failed to send command: {e}");
                        shutdown_tx.send(())?;
                        break;
                    }
                }
                if let cli::SubCommand::Exit = cli_command.sub_command {
                    break;
                }
            }
            Err(e) => {
                println!("Invalid command: {e}");
            }
        }
    }

    println!("Exiting CLI");
    shutdown_tx.send(())?;
    Ok(())
}
