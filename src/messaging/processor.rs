use crate::messaging::message::{Msg, MsgData, MsgMetaData};
use crate::tools::{time::TimeProvider, uuid::IdProvider};
use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel;

pub struct Processor<A, I, T>
where
    A: Actor,
    I: IdProvider,
    T: TimeProvider,
{
    pub input: channel::Receiver<Msg>,
    pub output: channel::Sender<Msg>,
    pub is_filter: bool,
    pub actor: A,
    pub id_provider: I,
    pub time_provider: T,
}

impl<A, I, T> Processor<A, I, T>
where
    A: Actor,
    I: IdProvider,
    T: TimeProvider,
{
    pub async fn start(mut self) -> Result<()> {
        loop {
            let e = self.input.recv().expect("open channel");
            if let MsgData::Shutdown = &e.data {
                break;
            };
            let mut msgs: Vec<Msg> = self
                .actor
                .act(&e)
                .await?
                .into_iter()
                .map(|msg| Msg {
                    data: msg,
                    metadata: MsgMetaData {
                        id: self.id_provider.new_random(),
                        created: self.time_provider.now(),
                        correlation_id: e.metadata.correlation_id,
                        causation_id: e.metadata.id,
                    },
                })
                .collect();
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
    async fn act(&mut self, msg: &Msg) -> Result<Vec<MsgData>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        messaging::message::PriceUpdated,
        tools::{time::tests::MockTimeProvider, uuid::tests::MockUuidProvider},
    };
    use crossbeam::channel::unbounded;
    use pretty_assertions::assert_eq;

    pub struct MockActor {}

    #[async_trait]
    impl Actor for MockActor {
        async fn act(&mut self, _: &Msg) -> Result<Vec<MsgData>> {
            Ok(vec![MsgData::AveragePriceUpdated(PriceUpdated {
                ..Default::default()
            })])
        }
    }

    fn new_processor(
        is_filter: bool,
    ) -> (
        Processor<MockActor, MockUuidProvider, MockTimeProvider>,
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
                id_provider: MockUuidProvider::new(),
                time_provider: MockTimeProvider::new(),
            },
            in_s,
            out_r,
        )
    }

    #[async_std::test]
    async fn processor_should_exit_if_shutdown_received() {
        let (processor, in_s, _) = new_processor(false);
        in_s.send(Msg {
            data: MsgData::Shutdown,
            metadata: MsgMetaData {
                ..Default::default()
            },
        })
        .unwrap();
        processor.start().await.unwrap();
        assert!(true);
    }

    #[async_std::test]
    async fn processor_should_output_input_events_if_not_filtered() {
        let (processor, in_s, out_r) = new_processor(false);
        let expected_msg = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            ..Default::default()
        }));
        in_s.send(expected_msg.clone()).unwrap();
        in_s.send(Msg::with_data(MsgData::Shutdown)).unwrap();
        processor.start().await.unwrap();
        let actual_msg_1 = out_r.recv().unwrap();
        assert_eq!(expected_msg, actual_msg_1);

        let actual_msg_2 = out_r.recv().unwrap();
        let expected_msg_2 = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            ..Default::default()
        }));
        assert_eq!(expected_msg_2, actual_msg_2);
    }

    #[async_std::test]
    async fn processor_should_not_output_input_events_if_filtered() {
        let (processor, in_s, out_r) = new_processor(true);
        let msg = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            ..Default::default()
        }));
        in_s.send(msg).unwrap();
        in_s.send(Msg::with_data(MsgData::Shutdown)).unwrap();
        processor.start().await.unwrap();
        let expected_msg = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            ..Default::default()
        }));
        let actual_msg = out_r.recv().unwrap();
        assert_eq!(expected_msg, actual_msg);
    }

    #[async_std::test]
    async fn processor_adds_metadata_for_new_msg() {
        let (processor, in_s, out_r) = new_processor(true);
        let msg = Msg {
            data: MsgData::LivePriceUpdated(PriceUpdated {
                ..Default::default()
            }),
            metadata: MsgMetaData {
                id: uuid::Uuid::from_u128(8),
                causation_id: uuid::Uuid::from_u128(7),
                correlation_id: uuid::Uuid::from_u128(7),
                created: 0,
            },
        };
        in_s.send(msg).unwrap();
        in_s.send(Msg::with_data(MsgData::Shutdown)).unwrap();
        processor.start().await.unwrap();
        let expected_msg = Msg {
            data: MsgData::AveragePriceUpdated(PriceUpdated {
                ..Default::default()
            }),
            metadata: MsgMetaData {
                id: uuid::Uuid::from_u128(0),
                causation_id: uuid::Uuid::from_u128(8),
                correlation_id: uuid::Uuid::from_u128(7),
                created: 0,
            },
        };
        let actual_msg = out_r.recv().unwrap();
        assert_eq!(expected_msg, actual_msg);
    }

    #[async_std::test]
    async fn processor_adds_different_metadata_for_new_msg() {
        let (processor, in_s, out_r) = new_processor(true);
        let msg = Msg {
            data: MsgData::LivePriceUpdated(PriceUpdated {
                ..Default::default()
            }),
            metadata: MsgMetaData {
                id: uuid::Uuid::from_u128(7),
                causation_id: uuid::Uuid::from_u128(6),
                correlation_id: uuid::Uuid::from_u128(6),
                created: 0,
            },
        };
        in_s.send(msg).unwrap();
        in_s.send(Msg::with_data(MsgData::Shutdown)).unwrap();
        processor.start().await.unwrap();
        let expected_msg = Msg {
            data: MsgData::AveragePriceUpdated(PriceUpdated {
                ..Default::default()
            }),
            metadata: MsgMetaData {
                id: uuid::Uuid::from_u128(0),
                causation_id: uuid::Uuid::from_u128(7),
                correlation_id: uuid::Uuid::from_u128(6),
                created: 0,
            },
        };
        let actual_msg = out_r.recv().unwrap();
        assert_eq!(expected_msg, actual_msg);
    }
}
