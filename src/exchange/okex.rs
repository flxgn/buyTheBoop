use websocket::{OwnedMessage, WebSocketError};
use flate2::read::DeflateDecoder;
use log::{info, error};
use std::io::prelude::Read;
use websocket::client::sync::Client;
use websocket::stream::sync::NetworkStream;
use websocket::{ClientBuilder, Message};
use serde::{Deserialize, Serialize};
use crate::exchange::{Exchange, ExchangeStreamEvent, Subscription, Pair, MarketOrder, OrderType, Order, Assets};
use serde_json::Value;
use uuid::Uuid;
use std::collections::HashMap;
use math::round;
use cast::i8;
use crossbeam::channel::{Sender, Receiver, unbounded};
use sha2::Sha256;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use chrono::prelude::{Utc, SecondsFormat};
use hmac::{Hmac, Mac};
use std::{env, thread};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use crate::tools::networking::HttpClient;

lazy_static! {
    static ref API_KEY: String = env::var("OKEX_API_KEY").unwrap();
    static ref API_SECRET: String = env::var("OKEX_API_SECRET").unwrap();
    static ref API_PASSPHRASE: String = env::var("OKEX_API_PASSPHRASE").unwrap();
}

static MOCK_SENDING: bool = true;
static WEBSOCKET_CHUNK_SIZE: usize = 100;

#[derive(Debug)]
pub struct Okex {
    size_increments: HashMap<Uuid, i8>
}

impl Okex {
    pub async fn new<C>(client: C) -> Okex where C: HttpClient {
        Okex {
            size_increments: create_size_increments_map().await,
        }
    }
}

async fn create_size_increments_map() -> HashMap<Uuid, i8> {
    let pair_details = reqwest::get("https://www.okex.com/api/spot/v3/instruments").await.unwrap().text().await.unwrap();
    let pair_details: Vec<PairDetail> = serde_json::from_str(&pair_details).unwrap();
    pair_details.iter().fold(HashMap::new(), |mut acc, pair_detail| {
        let id = Uuid::new_v3(&Uuid::NAMESPACE_OID, pair_detail.instrument_id.as_bytes());
        let size_increment = calculate_size_increment(&pair_detail.size_increment);
        acc.insert(id, size_increment);
        acc
    })
}

fn calculate_size_increment(size_increment: &String) -> i8 {
    let bytes = size_increment.as_bytes();
    let position = match bytes.iter().position(|x| x == &".".as_bytes()[0]) {
        Some(p) => p,
        None => return 0,
    };
    i8(bytes.len() - position - 1).unwrap()
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct PairDetail {
    instrument_id: String,
    base_currency: String,
    quote_currency: String,
    min_size: String,
    size_increment: String,
    tick_size: String,
}

fn websocket_worker(mut client: Client<Box<dyn NetworkStream + Send>>,
                    message: String,
                    sender: Sender<Result<OwnedMessage, WebSocketError>>) {
    let message = Message::text(message);
    client.send_message(&message).unwrap();
    loop {
        let result = client.incoming_messages().next().unwrap();
        sender.send(result).unwrap();
    }
}

#[async_trait]
impl Exchange for Okex {
    async fn fetch_assets(&self) -> Result<Assets> {
        unimplemented!()
    } 

    async fn event_stream<'a>(&'a self) -> Box<dyn Iterator<Item=ExchangeStreamEvent> + 'a> {
        let (sender, receiver) = unbounded();
        for (client, message) in client_pool().await {
            let cloned_sender = sender.clone();
            thread::spawn(move || {
                websocket_worker(client, message, cloned_sender);
            });
        }
        let iterator = EventStream { receiver, intermediate_pair_store: HashMap::new() };
        Box::new(iterator)
    }

    async fn place_market_order(&mut self, order: &MarketOrder) -> Result<()> {
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let method = "POST";
        let request_path = "/api/spot/v3/orders";

        let side = match order.order_type {
            OrderType::Buy => "buy",
            OrderType::Sell => "sell",
        };

        let amount_name = match order.order_type {
            OrderType::Buy => "notional",
            OrderType::Sell => "size",
        };
        let instrument_id = vec![order.bid_currency.as_str(), order.ask_currency.as_str()].join("-");

        let rounded_amount = match order.order_type {
            OrderType::Buy => order.amount.to_string(),
            OrderType::Sell => {
                let instrument_uuid = Uuid::new_v3(&Uuid::NAMESPACE_OID, instrument_id.as_bytes());
                let size_increment: i8 = self.size_increments.get(&instrument_uuid).unwrap().clone();
                round::floor(order.amount, size_increment).to_string()
            }
        };

        let mut body = HashMap::new();
        body.insert("type", "market");
        body.insert("side", side);
        body.insert("instrument_id", &instrument_id);
        body.insert(amount_name, &rounded_amount);
        let body_str = serde_json::to_string(&body).unwrap();

        let mut signature_content = String::new();
        signature_content.push_str(&timestamp);
        signature_content.push_str(method);
        signature_content.push_str(request_path);
        signature_content.push_str(&body_str);

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(API_SECRET.as_bytes()).unwrap();
        mac.update(signature_content.as_bytes());
        let signature = mac.finalize().into_bytes();
        let base64_signature = base64::encode(&signature);

        let mut header_map = HeaderMap::new();
        header_map.insert("OK-ACCESS-KEY", HeaderValue::from_str(&API_KEY).unwrap());
        header_map.insert("OK-ACCESS-SIGN", HeaderValue::from_str(&base64_signature).unwrap());
        header_map.insert("OK-ACCESS-TIMESTAMP", HeaderValue::from_str(&timestamp).unwrap());
        header_map.insert("OK-ACCESS-PASSPHRASE", HeaderValue::from_str(&API_PASSPHRASE).unwrap());
        header_map.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::new();
        let mut complete_url = String::from("https://www.okex.com");
        complete_url.push_str(request_path);


        if MOCK_SENDING {
            match client
                .post(complete_url.as_str())
                .headers(header_map)
                .body(body_str)
                .build() {
                Ok(res) => {
                    info!("{:#?}", res.body());
                    Result::Ok(())
                }
                Err(e) => {
                    info!("{:#?}", e);
                    Err(e.into())
                }
            }
        } else {
            match client
                .post(complete_url.as_str())
                .headers(header_map)
                .body(body_str)
                .send()
                .await {
                Ok(mut res) => {
                    let res_text = res.text().await.unwrap();
                    info!("{}", res_text);
                    let res_json: Value = serde_json::from_str(&res_text).unwrap();
                    let success = res_json["result"].as_bool().unwrap();

                    if success {
                        Result::Ok(())
                    } else {
                        Err(anyhow!("Request unsuccessful"))
                    }
                }
                Err(e) => {
                    info!("{:#?}", e);
                    Err(e.into())
                }
            }
        }
    }
}

struct EventStream {
    receiver: Receiver<Result<OwnedMessage, WebSocketError>>,
    intermediate_pair_store: HashMap<Uuid, Pair>,
}

impl Iterator for EventStream {
    type Item = ExchangeStreamEvent;
    fn next(&mut self) -> Option<ExchangeStreamEvent> {
        let msg: OwnedMessage = match self.receiver.recv().unwrap() {
            Ok(m) => m,
            Err(_e) => return None,
        };
//        info!("Len: {}", self.receiver.len());
        match msg {
            OwnedMessage::Close(_) => {
                error!("recv Close");
                None
            }
            OwnedMessage::Binary(msg) => {
                let msg = deflate(&msg);
                let v: HashMap<String, Value> = serde_json::from_str(&msg).unwrap();
                if v.contains_key("event") {
                    if v.get("event").unwrap() == "subscribe" {
                        let channel = v.get("channel").unwrap().as_str().unwrap();
                        let pair: Vec<&str> = channel.split(":").collect();
                        let bid_ask: Vec<&str> = pair[1].split("-").collect();
                        let subscription = Subscription {
                            id: Uuid::new_v3(&Uuid::NAMESPACE_OID, pair[1].as_bytes()),
                            bid_currency: String::from(bid_ask[0]),
                            ask_currency: String::from(bid_ask[1]),
                        };
                        Some(ExchangeStreamEvent::Subscription(subscription))
                    } else {
                        println!("event");
                        None
                    }
                } else if v.contains_key("table") {
                    if v.get("table").unwrap() == "spot/depth" {
                        let value = v.get("data").unwrap();
                        let data = &value[0];
                        let bids = data["bids"].as_array().unwrap();

                        let bid_orders = parse_orders(bids);
                        let ask_orders = parse_orders(data["asks"].as_array().unwrap());
                        let instrument_id = data["instrument_id"].as_str().unwrap();
//                        println!("{} - {}", instrument_id, bids.len());
                        let id = Uuid::new_v3(&Uuid::NAMESPACE_OID, instrument_id.as_bytes());
                        let pair = create_updated_pair(&mut self.intermediate_pair_store,
                                                       id,
                                                       bid_orders,
                                                       ask_orders);
                        Some(ExchangeStreamEvent::Pair(pair))
                    } else {
                        println!("table");
                        None
                    }
                } else {
                    println!("Else");
                    None
                }
            }
            _s => None
        }
    }
}

// TODO: This function needs major refactoring. This should be done with a serde struct.
fn parse_orders(raw_orders: &Vec<Value>) -> Vec<Order> {
    raw_orders.iter()
        .map(|el| {
            let el = el.as_array().unwrap();
            Order {
                price: el[0].as_str().unwrap().parse::<f64>().unwrap(),
                amount: el[1].as_str().unwrap().parse::<f64>().unwrap(),
            }
        })
        .collect()
}

fn create_updated_pair(intermediate_pair_store: &mut HashMap<Uuid, Pair>,
                       id: Uuid,
                       bid_orders: Vec<Order>,
                       ask_orders: Vec<Order>) -> Pair {
    match intermediate_pair_store.get_mut(&id) {
        Some(p) => {
            bid_orders.iter().map(|order| {
                add_to_bid_orders(&mut p.bid_orders, *order);
            }).for_each(drop);
            ask_orders.iter().map(|order| {
                add_to_ask_orders(&mut p.ask_orders, *order);
            }).for_each(drop);
            p.clone()
        }
        None => {
            let pair = Pair { id, bid_orders, ask_orders };
            intermediate_pair_store.insert(id, pair.clone());
            pair
        }
    }
}

fn add_to_ask_orders(orders: &mut Vec<Order>, new_order: Order) {
    let position = orders.binary_search_by(|o| o.price.partial_cmp(&new_order.price).unwrap());
    add_to_orders(orders, new_order, &position)
}

fn add_to_bid_orders(orders: &mut Vec<Order>, new_order: Order) {
    let position = orders.binary_search_by(|o| new_order.price.partial_cmp(&o.price).unwrap());
    add_to_orders(orders, new_order, &position)
}

fn add_to_orders(orders: &mut Vec<Order>,
                 new_order: Order,
                 position: &Result<usize, usize>) {
    match position {
        Ok(pos) => {
            orders.remove(*pos);
            if new_order.amount > 0.0 {
                orders.insert(*pos, new_order)
            }
        }
        Err(pos) => {
            if new_order.amount > 0.0 {
                orders.insert(*pos, new_order)
            }
        }
    }
}

fn deflate(msg: &Vec<u8>) -> String {
    let mut decoder = DeflateDecoder::new(msg.as_slice());
    let mut s = String::new();
    decoder.read_to_string(&mut s).unwrap();
    s
}

pub async fn client_pool() -> Vec<(Client<Box<dyn NetworkStream + Send>>, String)> {
    let instruments = get_instruments().await;
    let messages = make_subscription_message(&instruments);
    let mut client_pool = Vec::new();
    for (i, message) in messages.iter().enumerate() {
        info!("Starting clients ({}/{})", i + 1, messages.len());
        let client = client();
        client_pool.push((client, String::from(message)));
    }
    client_pool
}

pub fn client() -> Client<Box<dyn NetworkStream + Send>> {
    ClientBuilder::new("wss://real.okex.com:10442/ws/v3")
        .expect("fail new ws client")
        .connect(None).unwrap()
}

async fn get_instruments() -> String {
    reqwest::get("https://www.okex.com/api/spot/v3/instruments/ticker").await.unwrap().text().await.unwrap()
}

fn make_subscription_message(instruments: &str) -> Vec<String> {
    let instruments = deserialize_instruments(&instruments);
    to_subscription_message(&instruments)
}

fn deserialize_instruments(instruments: &str) -> Vec<Instrument> {
    let instruments: Vec<Instrument> = serde_json::from_str(instruments).unwrap();
    instruments
}

fn to_subscription_message(instruments: &Vec<Instrument>) -> Vec<String> {
    instruments.chunks(WEBSOCKET_CHUNK_SIZE).into_iter()
        .fold(Vec::new(), |mut acc, instruments| {
            let args = instruments.iter()
                .fold(Vec::new(), |mut acc, instrument| {
                    acc.push(format!("spot/depth:{}", &instrument.instrument_id));
                    acc
                });
            acc.push(format!(r#"{{"op": "subscribe", "args": {:?}}}"#, args));
            acc
        })
}

// TODO: Is this still needed? Just one field.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Instrument {
    instrument_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::Order;

    #[test]
    fn calculate_size_increment_test() {
        assert_eq!(3, calculate_size_increment(&String::from("0.001")));
        assert_eq!(0, calculate_size_increment(&String::from("1")));
        assert_eq!(0, calculate_size_increment(&String::from("10")));
        assert_eq!(2, calculate_size_increment(&String::from("0.01")));
    }

    #[test]
    fn unit_test_deserialize_instrument() {
        let input = r#"[{"instrument_id":"LTC-BTC"},
                        {"instrument_id":"LTC-BTC"}]"#;
        let actual_instruments: Vec<Instrument> = serde_json::from_str(input).unwrap();

        assert_eq!(
            vec![Instrument { instrument_id: String::from("LTC-BTC") },
                 Instrument { instrument_id: String::from("LTC-BTC") }],
            actual_instruments)
    }

    #[test]
    fn test_make_subscription_message() {
        let instruments = r#"[{"instrument_id":"LTC-BTC"},
                              {"instrument_id":"ETH-USDT"}]"#;

        let actual_message = make_subscription_message(instruments);
        assert_eq!(vec![r#"{"op": "subscribe", "args": ["spot/depth:LTC-BTC", "spot/depth:ETH-USDT"]}"#],
                   actual_message)
    }

    #[test]
    fn unit_test_add_to_ask_orders() {
        let mut orders = vec![Order { price: 0.0, amount: 1.0 },
                              Order { price: 1.0, amount: 2.0 },
                              Order { price: 3.0, amount: 1.0 }];
        let new_order = Order { price: 2.0, amount: 1.0 };
        add_to_ask_orders(&mut orders, new_order);
        assert_eq!(vec![Order { price: 0.0, amount: 1.0 },
                        Order { price: 1.0, amount: 2.0 },
                        Order { price: 2.0, amount: 1.0 },
                        Order { price: 3.0, amount: 1.0 }],
                   orders);

        let mut orders = vec![Order { price: 0.0, amount: 1.0 },
                              Order { price: 1.0, amount: 2.0 },
                              Order { price: 2.0, amount: 1.0 }];
        let new_order = Order { price: 2.0, amount: 2.0 };
        add_to_ask_orders(&mut orders, new_order);
        assert_eq!(vec![Order { price: 0.0, amount: 1.0 },
                        Order { price: 1.0, amount: 2.0 },
                        Order { price: 2.0, amount: 2.0 }],
                   orders);

        let mut orders = vec![Order { price: 0.0, amount: 1.0 },
                              Order { price: 1.0, amount: 2.0 },
                              Order { price: 2.0, amount: 1.0 },
                              Order { price: 3.0, amount: 1.0 }];
        let new_order = Order { price: 2.0, amount: 0.0 };
        add_to_ask_orders(&mut orders, new_order);
        assert_eq!(vec![Order { price: 0.0, amount: 1.0 },
                        Order { price: 1.0, amount: 2.0 },
                        Order { price: 3.0, amount: 1.0 }],
                   orders);
    }

    #[test]
    fn unit_test_add_to_bid_orders() {
        let mut orders = vec![Order { price: 3.0, amount: 1.0 },
                              Order { price: 1.0, amount: 2.0 },
                              Order { price: 0.0, amount: 1.0 }];
        let new_order = Order { price: 2.0, amount: 1.0 };
        add_to_bid_orders(&mut orders, new_order);
        assert_eq!(vec![Order { price: 3.0, amount: 1.0 },
                        Order { price: 2.0, amount: 1.0 },
                        Order { price: 1.0, amount: 2.0 },
                        Order { price: 0.0, amount: 1.0 }],
                   orders);
    }

}