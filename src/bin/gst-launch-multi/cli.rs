use std::collections::VecDeque;

use clap::{Parser, Subcommand};

const DEFAULT_PIPELINE_NAME: &str = "pipeline_";

#[derive(Debug)]
pub(crate) struct CliArgs {
    pub(crate) app_config: AppConfig,
    pub(crate) pipeline_config: Vec<PipelineConfig>,
}

impl CliArgs {
    pub(crate) fn parse() -> Result<Self, clap::Error> {
        let args = std::env::args().collect::<Vec<_>>();

        CliArgs::parse_from_vec(args)
    }

    // pub(crate) fn parse_from_str(command: &str) -> Result<Self, clap::Error> {
    //     let args = command
    //         .split_terminator(&[' ', '\n'])
    //         .filter_map(|e| {
    //             if e.is_empty() {
    //                 None
    //             } else {
    //                 Some(e.to_string())
    //             }
    //         })
    //         .collect::<Vec<_>>();

    //     CliArgs::parse_from_vec(args)
    // }

    pub(crate) fn parse_from_vec(args: Vec<String>) -> Result<Self, clap::Error> {
        let mut pipeline_counter = 0;

        let mut args_groups = args.split(|e| e == "--pipeline").collect::<VecDeque<_>>();

        if args_groups.is_empty() {
            return Err(clap::Error::new(clap::error::ErrorKind::TooFewValues));
        }

        let app_args = args_groups.pop_front().unwrap().to_owned();
        let mut app_config = AppConfig::parse_from(app_args);

        let mut pipeline_configs: Vec<PipelineConfig> = Vec::new();

        // if app_config
        if !app_config.pipeline_spec_args.is_empty() {
            let mut pipeline_args = vec!["pipeline".to_string()];
            app_config
                .pipeline_spec_args
                .iter()
                .for_each(|e| pipeline_args.push(e.to_string()));
            app_config.pipeline_spec_args.clear();

            let mut pipeline_config = PipelineConfig::parse_from(pipeline_args);
            if pipeline_config.name == DEFAULT_PIPELINE_NAME {
                pipeline_config.name = format!("{DEFAULT_PIPELINE_NAME}{pipeline_counter}");
                pipeline_counter += 1;
            }

            pipeline_configs.push(pipeline_config);
        }

        while let Some(args) = args_groups.pop_front() {
            // Creates a new args vector where the first element is "pipeline"
            // This is to emulate the args vector coming from std::env::args where
            // the first element is the path to the executable.
            //
            // This allows calling the generated parse_from() method
            let mut pipeline_args = vec!["pipeline".to_string()];
            args.iter().for_each(|e| pipeline_args.push(e.to_string()));

            let mut pipeline_config = PipelineConfig::parse_from(pipeline_args);
            if pipeline_config.name == DEFAULT_PIPELINE_NAME {
                // FIXME: should control that every pipeline has a distinct name
                pipeline_config.name = format!("{DEFAULT_PIPELINE_NAME}{pipeline_counter}");
                pipeline_counter += 1;
            }

            pipeline_configs.push(pipeline_config);
        }

        Ok(CliArgs {
            app_config,
            pipeline_config: pipeline_configs,
        })
    }
}

#[derive(Parser, Debug)]
pub(crate) struct AppConfig {
    /// Application name
    #[clap(long, required = false, default_value = "gstreamer_transcoder")]
    pub(crate) app_name: String,

    /// OpenTelemetry URL
    #[clap(long, required = false, default_value = "grpc://localhost:4317")]
    pub(crate) opentelemetry_url: String,

    // Used to capture any other arguments. They are used to create
    // a PipelineConfig during the parsing process
    #[clap()]
    pub(crate) pipeline_spec_args: Vec<String>,
}

#[derive(Parser)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) sub_command: SubCommand,
}

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum SubCommand {
    AddPipeline(PipelineConfig),
    StopPipeline(StopPipelineCommand),
    SetProperty(SetPropertyCommand),
    SwitchPad(SwitchPadCommand),
    PushLatencyEvent(PushLatencyEventCommand),
    SetLatency(SetLatencyCommand),
    GetLatency(GetLatencyCommand),
    Exit,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct PipelineConfig {
    /// Pipeline name
    #[clap(long, required = false, default_value = DEFAULT_PIPELINE_NAME)]
    pub(crate) name: String,

    /// Pipeline spec
    #[clap(required = true)]
    pub(crate) spec: Vec<String>,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct StopPipelineCommand {
    #[clap(required = false)]
    pub(crate) pipelines: Vec<String>,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct SetPropertyCommand {
    #[clap(long, required = true)]
    pub(crate) pipeline: String,

    #[clap(long, required = true)]
    pub(crate) element: String,

    #[clap(long, required = true)]
    pub(crate) property: String,

    #[clap(long, required = true)]
    pub(crate) value: String,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct SwitchPadCommand {
    #[clap(long, required = true)]
    pub(crate) pipeline: String,

    #[clap(long, required = true)]
    pub(crate) element: String,

    #[clap(long, required = true)]
    pub(crate) pad: String,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct PushLatencyEventCommand {
    #[clap(long, required = true)]
    pub(crate) pipeline: String,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct SetLatencyCommand {
    #[clap(long, required = true)]
    pub(crate) pipeline: String,

    #[clap(long, required = false)]
    pub(crate) element: Option<String>,

    #[clap(long, required = true)]
    pub(crate) latency_ms: u64,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct GetLatencyCommand {
    #[clap(long, required = true)]
    pub(crate) pipeline: String,

    #[clap(long, required = false)]
    pub(crate) element: Option<String>,
}
