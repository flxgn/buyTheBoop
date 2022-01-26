use crate::messaging::message::Msg;
use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel;

pub struct Processor<T>
where
    T: Actor,
{
    pub input: channel::Receiver<Msg>,
    pub output: channel::Sender<Msg>,
    pub is_filter: bool,
    pub actor: T,
}

impl<T> Processor<T>
where
    T: Actor,
{
    pub async fn start(mut self) -> Result<()> {
        loop {
            let e = self.input.recv().expect("open channel");
            let mut msgs = match &e {
                Msg::Shutdown => break,
                msg => self.actor.act(msg).await?,
            };
            if !self.is_filter {
                msgs.insert(0, e)
            }
            for msg in msgs {
                self.output.send(msg).expect("open channel")
            }
        }
        Ok(())
    }
}

#[async_trait]
pub trait Actor {
    async fn act(&mut self, msg: &Msg) -> Result<Vec<Msg>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::message::PriceUpdated;
    use crossbeam::channel::unbounded;
    use pretty_assertions::assert_eq;

    pub struct MockActor {}

    #[async_trait]
    impl Actor for MockActor {
        async fn act(&mut self, msg: &Msg) -> Result<Vec<Msg>> {
            Ok(vec![Msg::AveragePriceUpdated(PriceUpdated {
                ..Default::default()
            })])
        }
    }

    fn new_processor(
        is_filter: bool,
    ) -> (
        Processor<MockActor>,
        channel::Sender<Msg>,
        channel::Receiver<Msg>,
    ) {
        let (in_s, in_r) = unbounded();
        let (out_s, out_r) = unbounded();
        (
            Processor {
                input: in_r,
                output: out_s,
                is_filter,
                actor: MockActor {},
            },
            in_s,
            out_r,
        )
    }

    #[async_std::test]
    async fn processor_should_exit_if_shutdown_received() {
        let (processor, in_s, _) = new_processor(false);
        in_s.send(Msg::Shutdown).unwrap();
        processor.start().await.unwrap();
        assert!(true);
    }

    #[async_std::test]
    async fn processor_should_output_input_events_if_not_filtered() {
        let (processor, in_s, out_r) = new_processor(false);
        let expected_msg = Msg::LivePriceUpdated(PriceUpdated {
            ..Default::default()
        });
        in_s.send(expected_msg.clone()).unwrap();
        in_s.send(Msg::Shutdown).unwrap();
        processor.start().await.unwrap();
        let actual_msg_1 = out_r.recv().unwrap();
        assert_eq!(expected_msg, actual_msg_1);

        let actual_msg_2 = out_r.recv().unwrap();
        let expected_msg_2 = Msg::AveragePriceUpdated(PriceUpdated {
            ..Default::default()
        });
        assert_eq!(expected_msg_2, actual_msg_2);
    }

    #[async_std::test]
    async fn processor_should_not_output_input_events_if_filtered() {
        let (processor, in_s, out_r) = new_processor(true);
        let msg = Msg::LivePriceUpdated(PriceUpdated {
            ..Default::default()
        });
        in_s.send(msg).unwrap();
        in_s.send(Msg::Shutdown).unwrap();
        processor.start().await.unwrap();
        let expected_msg = Msg::AveragePriceUpdated(PriceUpdated {
            ..Default::default()
        });
        let actual_msg = out_r.recv().unwrap();
        assert_eq!(expected_msg, actual_msg);
    }
}
