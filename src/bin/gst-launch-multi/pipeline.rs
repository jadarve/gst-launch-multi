use std::sync::Arc;

use crate::cli;

use anyhow::Result;
use gst::prelude::{
    Cast, ElementExt, ElementExtManual, GObjectExtManualGst, GstBinExt, GstBinExtManual,
    GstObjectExt, ObjectExt, PadExtManual,
};

use gst::glib;

use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub(crate) struct PipelineSharedSettings {
    pub(crate) cli_stopped: bool,
}

#[derive(Clone)]
pub(crate) struct Pipeline {
    /// the configuration used to create the pipeline
    pub(crate) config: cli::PipelineConfig,

    /// The GStreamer pipeline reference
    pub(crate) pipeline: gst::Pipeline,

    /// TODO: I should use a tokio::sync::Mutex instead of a std::sync::Mutex as mixing a std Mutex
    /// in a Tokio task may lead to blocking the whole Tokio runtime running several tasks.
    /// However, this mutex is needed inside a GStreamer PadProbe function.
    pub(crate) shared_settings: std::sync::Arc<std::sync::Mutex<PipelineSharedSettings>>,
}

impl Pipeline {

    pub(crate) fn new(
        config: &cli::PipelineConfig,
        clock: Option<&gst::Clock>,
        basetime: &gst::ClockTime,
    ) -> Result<Arc<Self>> {
        let pipeline_spec = &config.spec.join(" ");

        let pipeline = gst::parse::launch(pipeline_spec)?
            .downcast::<gst::Pipeline>()
            .map_err(|e| anyhow::anyhow!("Unable to downcast pipeline: {e:?}"))?;

        pipeline.set_property_from_str("name", &config.name);

        // Set the clock and basetime. Every pipeline created by the application
        // must share the same clock and basetime.
        pipeline.set_clock(clock)?;
        pipeline.set_start_time(gst::ClockTime::NONE);
        pipeline.set_base_time(basetime.to_owned());

        // let p = 

        Ok(Arc::new(Self {
            config: config.to_owned(),
            pipeline,
            shared_settings: std::sync::Arc::new(std::sync::Mutex::new(PipelineSharedSettings {
                cli_stopped: false,
            })),
        }))
    }

    pub(crate) fn start(&self) -> Result<()> {
        self.pipeline.set_state(gst::State::Playing)?;
        Ok(())
    }

    pub(crate) fn start_arc(self: &Arc<Self>) -> Result<()> {
        self.pipeline.set_state(gst::State::Playing)?;
        Ok(())
    }

    pub(crate) fn stop(&self) -> Result<()> {
        self.pipeline.set_state(gst::State::Null)?;
        Ok(())
    }

    pub(crate) fn stop_arc(self: &Arc<Self>) -> Result<()> {
        self.pipeline.set_state(gst::State::Null)?;
        Ok(())
    }

    pub(crate) async fn run(self: &Arc<Self>,
        mut _shutdown_receiver: broadcast::Receiver<()>,
        mut command_receiver: broadcast::Receiver<cli::SubCommand>) -> Result<()> {

        // first, change the pipeline state to PLAYING
        self.start()?;

        // holds references to the various tasks needed to handle
        // the lifetime of the pipeline
        let mut task_set = tokio::task::JoinSet::new();

        ///////////////////////////////////////////////////////
    // CLI command task
    let pipeline_command_clone = self.clone();
    task_set.spawn(async move {
        let pipeline = pipeline_command_clone;

        while let Ok(command) = command_receiver.recv().await {
            match command {
                cli::SubCommand::SetProperty(args) => {
                    if pipeline.config.name == args.pipeline {
                        println!("Set property: {}: {:?}", pipeline.config.name, args);
                        if let Some(element) = pipeline.pipeline.by_name(&args.element) {
                            element.set_property_from_str(&args.property, &args.value);
                        }
                    }
                }
                cli::SubCommand::SwitchPad(args) => {
                    if pipeline.config.name == args.pipeline {
                        println!("Switch pad: {}: {:?}", pipeline.config.name, args);
                        if let Some(element) = pipeline.pipeline.by_name(&args.element) {
                            // TODO: should check the class of the element is "input-selector"
                            if let Some(pad) = element.static_pad(&args.pad) {
                                element.set_property("active-pad", pad);
                            }
                        }
                    }
                }
                cli::SubCommand::StopPipeline(args) => {
                    let found = args
                        .pipelines
                        .iter()
                        .find(|pipeline_name| pipeline_name.as_str() == pipeline.config.name);

                    if found.is_some() {
                        println!("Stop pipeline: {}", pipeline.config.name);
                        // send an EOS event which is processed in the bus event loop task (below)
                        // where the pipeline is stopped and.
                        let eos_event = gst::event::Eos::new();

                        // mark the pipeline as being stopped manually via CLI
                        {
                            let mut shared_settings = pipeline.shared_settings.lock().unwrap();
                            shared_settings.cli_stopped = true;
                        }

                        let _ = pipeline.pipeline.send_event(eos_event);

                        // break the CLI loop, completing this task
                        break;
                    }
                }
                cli::SubCommand::Exit => {
                    // send an EOS event which is processed in the bus event loop task (below)
                    // where the pipeline is stopped.
                    let eos_event = gst::event::Eos::new();
                    let _ = pipeline.pipeline.send_event(eos_event);

                    // break the CLI loop, completing this task
                    break;
                }
                cli::SubCommand::PushLatencyEvent(args) => {
                    if pipeline.config.name == args.pipeline {
                        println!("Push latency event: {}", pipeline.config.name);
                        // let latency_event = gst::event::Latency::new(gst::ClockTime::from_mseconds(args.latency_ms));

                        let msg = gst::message::Latency::builder().build();
                        let _ = pipeline.pipeline.post_message(msg);

                        
                    }
                }
                cli::SubCommand::SetLatency(args) => {
                    if pipeline.config.name == args.pipeline {

                        let latency_event = gst::event::Latency::new(gst::ClockTime::from_mseconds(args.latency_ms));

                        if let Some(element_name) = args.element {
                            if let Some(element) = pipeline.pipeline.by_name(&element_name) {
                                let _ = element.send_event(latency_event);
                            }
                        }
                        else {
                            let _ = pipeline.pipeline.send_event(latency_event);
                        }
                    }
                }
                cli::SubCommand::GetLatency(args) => {
                    if pipeline.config.name == args.pipeline {
                        println!("Get latency: {}", pipeline.config.name);
                        
                        let mut query = gst::query::Latency::new();

                        if let Some(element_name) = args.element {
                            if let Some(element) = pipeline.pipeline.by_name(&element_name) {
                                let _ = element.query(&mut query);
                            }
                        } else {
                            let _ = pipeline.pipeline.query(query.query_mut());
                        }
                        

                        if let gst::QueryView::Latency(latency) = query.view() {
                            let (is_live, min_latency, max_latency) = latency.result();
                            println!("Latency: is_live: {is_live}, min_latency: {min_latency}, max_latency: {max_latency:?}");
                        }

                        // match query.view() {
                        //     gst::QueryView::Latency(latency) => {
                        //         println!("Latency: {:?}", latency);
                        //     }
                        //     _ => {}
                        // }
                        // println!("Latency: {:?}", query);
                    }
                }
                _ => {}
            }
        }
    });

    ///////////////////////////////////////////////////////
    // Bus message handling task
    let pipeline_bus_task_clone = self.clone();
    task_set.spawn(async move {
        
        let pipeline = pipeline_bus_task_clone;

        // search for all intersrc elements and add a probe to the src pad to handle EOS events
        for child in pipeline
            .pipeline
            .iterate_all_by_element_factory_name("intersrc")
            .into_iter()
            .flatten()
        {

            let element_name = child.name().to_string();
            if let Some(src_pad) = child.static_pad("src") {
                let pipeline_name = pipeline.config.name.clone();
                let pipeline_clone = pipeline.clone();

                src_pad.add_probe(
                    gst::PadProbeType::EVENT_DOWNSTREAM,
                    move |_pad, probe_info| {
                        
                        // the pipeline_clone is moved into this closure
                        match probe_info.data {
                            Some(gst::PadProbeData::Event(ref event)) => match event.view() {
                                gst::EventView::Eos(_) => {

                                    let pad_return = {
                                        let shared_settings = pipeline_clone.shared_settings.lock().unwrap();
                                        if shared_settings.cli_stopped {
                                            gst::PadProbeReturn::Ok
                                        } else {
                                            
                                            ///////////////////////////////////////////////////////
                                            // restart the intersrc bin                                            
                                            let child_clone = child.clone();
                                            glib::idle_add(move || {
                                                child_clone.set_state(gst::State::Null).unwrap();
                                                child_clone.set_state(gst::State::Playing).unwrap();
                                                // no need to call the closure again
                                                glib::ControlFlow::Break
                                            });

                                            gst::PadProbeReturn::Handled
                                        }
                                    };

                                    println!(
                                        "Pipeline: {pipeline_name} EOS on intersrc element: {element_name}, pad_return: {pad_return:?}",
                                    );
                                    pad_return
                                }
                                _ => gst::PadProbeReturn::Ok,
                            },
                            _ => gst::PadProbeReturn::Ok,
                        }

                    },
                );
            }
        }

        if let Some(bus) = pipeline.pipeline.bus() {
            for msg in bus.iter_timed(gst::ClockTime::NONE) {
                match msg.view() {
                    gst::MessageView::Eos(msg) => {
                        println!(
                            "Pipeline: {}: End-Of-Stream reached: {:?}",
                            pipeline.config.name, msg
                        );
                        // stop the pipeline and break the loop
                        let _ = pipeline.stop();
                        break;
                    }
                    gst::MessageView::Error(err) => {
                        println!(
                            "Pipeline: {}: Error received from element {:?}",
                            pipeline.config.name,
                            err.message()
                        );
                    }
                    gst::MessageView::Latency(msg) => {
                        println!(
                            "Pipeline: {}: Latency message received {msg:?}",
                            pipeline.config.name
                        );

                        let mut query = gst::query::Latency::new();
                        let _ = pipeline.pipeline.query(query.query_mut());
                        if let gst::QueryView::Latency(latency) = query.view() {
                            let (is_live, min_latency, max_latency) = latency.result();
                            println!("Pipeline: {}: Latency before recalculation : is_live: {is_live}, min_latency: {min_latency}, max_latency: {max_latency:?}", pipeline.config.name);
                        }

                        let _ = pipeline.pipeline.recalculate_latency();

                        let _ = pipeline.pipeline.query(query.query_mut());
                        if let gst::QueryView::Latency(latency) = query.view() {
                            let (is_live, min_latency, max_latency) = latency.result();
                            println!("Pipeline: {}: Latency after recalculation : is_live: {is_live}, min_latency: {min_latency}, max_latency: {max_latency:?}", pipeline.config.name);
                        }
                    }
                    _ => {}
                }
            }
        } else {
            println!(
                "ERROR: unable to get bus for pipeline: {}",
                pipeline.config.name
            );
        }
    });

    // wait for all the tasks controlling the pipeline to finish
    while task_set.join_next().await.is_some() {}
        Ok(())
    }
}
