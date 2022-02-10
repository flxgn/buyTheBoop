use uuid::Uuid;

pub type Timestamp = i128;
pub type AccurateTimestamp = u128;
pub type Price = f64;
pub type PairId = &'static str;
pub type EventId = Uuid;
pub type MessageId = Uuid;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct PriceUpdated {
    pub id: EventId,
    pub pair_id: PairId,
    pub datetime: Timestamp,
    pub price: Price,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum OrderType {
    Buy,
    Sell,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Order {
    pub base: String,
    pub quote: String,
    pub amount: f64,
}

#[derive(Debug, PartialEq, Clone)]
pub enum MsgData {
    LivePriceUpdated(PriceUpdated),
    AveragePriceUpdated(PriceUpdated),
    Bought(Order),
    Sold(Order),
    Buy,
    Sell,
    Shutdown,
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct MsgMetaData {
    pub id: MessageId,
    pub created: AccurateTimestamp,
    pub correlation_id: MessageId,
    pub causation_id: MessageId,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Msg {
    pub data: MsgData,
    pub metadata: MsgMetaData,
}

impl Msg {
    pub fn with_data(data: MsgData) -> Self {
        Msg {
            data,
            metadata: MsgMetaData {
                ..Default::default()
            },
        }
    }
}
