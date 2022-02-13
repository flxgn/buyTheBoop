use crate::messaging::message::{Msg, MsgData, MsgMetaData};
use crate::tools::{time::TimeProvider, uuid::IdProvider};
use anyhow::Result;
use async_std::task;
use async_trait::async_trait;
use crossbeam::channel;
use crossbeam::channel::unbounded;

struct Processor<I, T>
where
    I: IdProvider,
    T: TimeProvider,
{
    input: channel::Receiver<Msg>,
    output: channel::Sender<Msg>,
    is_filter: bool,
    actor: Box<dyn Actor + Send>,
    id_provider: I,
    time_provider: T,
}

impl<I, T> Processor<I, T>
where
    I: IdProvider,
    T: TimeProvider,
{
    pub async fn start(mut self) -> Result<()> {
        loop {
            let e = self.input.recv().expect("open channel");
            if let MsgData::Shutdown = &e.data {
                self.output.send(e).expect("open channel");
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

pub struct ActorChain<I, T>
where
    I: IdProvider,
    T: TimeProvider,
{
    processors: Vec<Processor<I, T>>,
    time_provider: T,
    id_provider: I,
    previous_receiver_channel: channel::Receiver<Msg>,
    receiver_channel: channel::Receiver<Msg>,
    sender_channel: channel::Sender<Msg>,
}

impl<I: 'static, T: 'static> ActorChain<I, T>
where
    I: IdProvider + Clone + Send,
    T: TimeProvider + Clone + Send,
{
    pub fn new(time_provider: T, id_provider: I, channel: channel::Receiver<Msg>) -> Self {
        let (sender, receiver) = unbounded();
        ActorChain {
            processors: vec![],
            time_provider,
            id_provider,
            previous_receiver_channel: channel,
            receiver_channel: receiver,
            sender_channel: sender,
        }
    }
    pub fn add<A: Actor + Send + 'static>(mut self, actor: A) -> Self {
        let processor = Processor {
            input: self.previous_receiver_channel,
            output: self.sender_channel,
            // TODO: Fix this to be either removed, inside actor, or configurable from outside
            is_filter: false,
            actor: Box::new(actor),
            id_provider: self.id_provider.clone(),
            time_provider: self.time_provider.clone(),
        };
        self.processors.push(processor);
        let (new_sender, new_receiver) = unbounded();
        self.sender_channel = new_sender;
        self.previous_receiver_channel = self.receiver_channel;
        self.receiver_channel = new_receiver;
        self
    }
    pub async fn start(self) -> channel::Receiver<Msg> {
        for processor in self.processors {
            task::spawn(async move {
                processor.start().await.unwrap();
            });
        }
        self.previous_receiver_channel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        messaging::message::MsgMetaData,
        tools::{time::tests::MockTimeProvider, uuid::tests::MockUuidProvider},
    };
    use uuid::Uuid;

    use pretty_assertions::assert_eq;

    pub struct MockActor {}

    #[async_trait]
    impl Actor for MockActor {
        async fn act(&mut self, _: &Msg) -> Result<Vec<MsgData>> {
            Ok(vec![MsgData::Buy])
        }
    }

    fn new_processor(
        is_filter: bool,
    ) -> (
        Processor<MockUuidProvider, MockTimeProvider>,
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
                actor: Box::new(MockActor {}),
                id_provider: MockUuidProvider::new(),
                time_provider: MockTimeProvider::new(),
            },
            in_s,
            out_r,
        )
    }

    #[async_std::test]
    async fn processor_should_exit_if_shutdown_received() {
        let (processor, in_s, _out_r) = new_processor(false);
        in_s.send(Msg::with_data(MsgData::Shutdown)).unwrap();
        processor.start().await.unwrap();
        assert!(true);
    }

    #[async_std::test]
    async fn processor_should_emit_received_shutdown() {
        let (processor, in_s, out_r) = new_processor(false);
        let expected_msg = Msg::with_data(MsgData::Shutdown);

        in_s.send(expected_msg.clone()).unwrap();
        processor.start().await.unwrap();

        let actual_message = out_r.recv().unwrap();
        assert_eq!(expected_msg, actual_message);
    }

    #[async_std::test]
    async fn processor_should_output_input_events_if_not_filtered() {
        let (processor, in_s, out_r) = new_processor(false);
        let expected_msg = Msg::with_data(MsgData::Sell);

        in_s.send(expected_msg.clone()).unwrap();
        in_s.send(Msg::with_data(MsgData::Shutdown)).unwrap();
        processor.start().await.unwrap();

        let actual_msg_1 = out_r.recv().unwrap();
        assert_eq!(expected_msg, actual_msg_1);
        let actual_msg_2 = out_r.recv().unwrap();
        let expected_msg_2 = Msg::with_data(MsgData::Buy);
        assert_eq!(expected_msg_2, actual_msg_2);
    }

    #[async_std::test]
    async fn processor_should_not_output_input_events_if_filtered() {
        let (processor, in_s, out_r) = new_processor(true);
        let msg = Msg::with_data(MsgData::Sell);

        in_s.send(msg).unwrap();
        in_s.send(Msg::with_data(MsgData::Shutdown)).unwrap();
        processor.start().await.unwrap();

        let expected_msg = Msg::with_data(MsgData::Buy);
        let actual_msg = out_r.recv().unwrap();
        assert_eq!(expected_msg, actual_msg);
    }

    #[async_std::test]
    async fn processor_adds_metadata_for_new_msg() {
        let (processor, in_s, out_r) = new_processor(true);
        let msg = Msg {
            data: MsgData::Sell,
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
            data: MsgData::Buy,
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
            data: MsgData::Sell,
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
            data: MsgData::Buy,
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

    #[async_std::test]
    async fn actor_chain_starts_up_without_actor() {
        let (sender, receiver) = unbounded();
        let output = ActorChain::new(MockTimeProvider::new(), MockUuidProvider::new(), receiver)
            .start()
            .await;
        let expected_msg = Msg::with_data(MsgData::Shutdown);
        sender.send(expected_msg.clone()).unwrap();
        let actual_msg = output.recv().unwrap();
        assert_eq!(expected_msg, actual_msg);
    }

    #[async_std::test]
    async fn actor_chain_starts_up_with_simple_actor() {
        let (sender, receiver) = unbounded();
        let output = ActorChain::new(MockTimeProvider::new(), MockUuidProvider::new(), receiver)
            .add(MockActor {})
            .start()
            .await;
        let expected_msg = Msg::with_data(MsgData::Shutdown);
        sender.send(expected_msg.clone()).unwrap();
        let actual_msg = output.recv().unwrap();
        assert_eq!(expected_msg, actual_msg);
    }

    #[async_std::test]
    async fn actor_chain_calls_internal_actor() {
        let (sender, receiver) = unbounded();
        let output = ActorChain::new(MockTimeProvider::new(), MockUuidProvider::new(), receiver)
            .add(MockActor {})
            .start()
            .await;

        sender.send(Msg::with_data(MsgData::Sell)).unwrap();
        sender.send(Msg::with_data(MsgData::Shutdown)).unwrap();

        let messages: Vec<Msg> = output.iter().collect();
        assert_eq!(
            vec![
                Msg::with_data(MsgData::Sell),
                Msg::with_data(MsgData::Buy),
                Msg::with_data(MsgData::Shutdown)
            ],
            messages
        );
    }

    #[async_std::test]
    async fn actor_chain_calls_multiple_internal_actor() {
        let (sender, receiver) = unbounded();
        let output = ActorChain::new(MockTimeProvider::new(), MockUuidProvider::new(), receiver)
            .add(MockActor {})
            .add(MockActor {})
            .start()
            .await;

        sender.send(Msg::with_data(MsgData::Sell)).unwrap();
        sender.send(Msg::with_data(MsgData::Shutdown)).unwrap();

        let messages: Vec<Msg> = output.iter().collect();
        assert_eq!(
            vec![
                Msg::with_data(MsgData::Sell),
                Msg::with_data(MsgData::Buy),
                Msg::with_data(MsgData::Buy),
                Msg {
                    data: MsgData::Buy,
                    metadata: MsgMetaData {
                        id: Uuid::from_u128(1),
                        created: 1,
                        ..Default::default()
                    }
                },
                Msg::with_data(MsgData::Shutdown)
            ],
            messages
        );
    }
}
