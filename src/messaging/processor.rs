use crate::messaging::message::Msg;
use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel;

pub struct PureProcessor<'a, T>
where
    T: Aggregator<'a>,
{
    pub input: channel::Receiver<Msg<'a>>,
    pub output: channel::Sender<Msg<'a>>,
    pub is_filter: bool,
    pub aggregator: T,
}

impl<'a, T> PureProcessor<'a, T>
where
    T: Aggregator<'a>,
{
    pub fn start(mut self) {
        loop {
            let e = self.input.recv().expect("open channel");
            let mut msgs = match &e {
                Msg::Shutdown => break,
                msg => self.aggregator.aggregate(msg),
            };
            if !self.is_filter {
                msgs.insert(0, e)
            }
            for msg in msgs {
                self.output.send(msg).expect("open channel")
            }
        }
    }
}

pub trait Aggregator<'a> {
    fn aggregate(&mut self, msg: &Msg<'a>) -> Vec<Msg<'a>>;
}

pub struct EffectProcessor<'a, T>
where
    T: Executor<'a>,
{
    pub input: channel::Receiver<Msg<'a>>,
    pub output: channel::Sender<Msg<'a>>,
    pub executor: T,
}

impl<'a, T> EffectProcessor<'a, T>
where
    T: Executor<'a>,
{
    pub async fn start(mut self) -> Result<()> {
        loop {
            let e = self.input.recv().expect("open channel");
            match &e {
                Msg::Shutdown => break,
                msg => self.executor.execute(msg).await?,
            };
            self.output.send(e).expect("open channel");
        }
        Ok(())
    }
}

#[async_trait]
pub trait Executor<'a> {
    async fn execute(&mut self, msg: &Msg<'a>) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::message::PriceUpdated;
    use crossbeam::channel::unbounded;
    use pretty_assertions::assert_eq;

    pub struct MockAggregator {}

    impl<'a> Aggregator<'a> for MockAggregator {
        fn aggregate(&mut self, msg: &Msg<'a>) -> Vec<Msg<'a>> {
            vec![Msg::AveragePriceUpdated(PriceUpdated {
                ..Default::default()
            })]
        }
    }

    fn new_processor<'a>(
        is_filter: bool,
    ) -> (
        PureProcessor<'a, MockAggregator>,
        channel::Sender<Msg<'a>>,
        channel::Receiver<Msg<'a>>,
    ) {
        let (in_s, in_r) = unbounded();
        let (out_s, out_r) = unbounded();
        (
            PureProcessor {
                input: in_r,
                output: out_s,
                is_filter,
                aggregator: MockAggregator {},
            },
            in_s,
            out_r,
        )
    }

    #[test]
    fn processor_should_exit_if_shutdown_received() {
        let (processor, in_s, _) = new_processor(false);
        in_s.send(Msg::Shutdown).unwrap();
        processor.start();
        assert!(true);
    }

    #[test]
    fn processor_should_output_input_events_if_not_filtered() {
        let (processor, in_s, out_r) = new_processor(false);
        let expected_msg = Msg::LivePriceUpdated(PriceUpdated {
            ..Default::default()
        });
        in_s.send(expected_msg.clone()).unwrap();
        in_s.send(Msg::Shutdown).unwrap();
        processor.start();
        let actual_msg_1 = out_r.recv().unwrap();
        assert_eq!(expected_msg, actual_msg_1);

        let actual_msg_2 = out_r.recv().unwrap();
        let expected_msg_2 = Msg::AveragePriceUpdated(PriceUpdated {
            ..Default::default()
        });
        assert_eq!(expected_msg_2, actual_msg_2);
    }

    #[test]
    fn processor_should_not_output_input_events_if_filtered() {
        let (processor, in_s, out_r) = new_processor(true);
        let msg = Msg::LivePriceUpdated(PriceUpdated {
            ..Default::default()
        });
        in_s.send(msg).unwrap();
        in_s.send(Msg::Shutdown).unwrap();
        processor.start();
        let expected_msg = Msg::AveragePriceUpdated(PriceUpdated {
            ..Default::default()
        });
        let actual_msg = out_r.recv().unwrap();
        assert_eq!(expected_msg, actual_msg);
    }
}
