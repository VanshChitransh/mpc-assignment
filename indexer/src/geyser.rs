use std::pin::Pin;
use std::task::{Context, Poll};
use futures_util::{Sink, Stream};
use tonic::{Response, Status};
use tokio::sync::mpsc;
use tracing::{info, debug, warn};

// Mock types for Yellowstone GRPC since we don't have the actual dependency
pub mod yellowstone_grpc_proto {
    pub mod prelude {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum CommitmentLevel {
            Processed = 0,
            Confirmed = 1,
            Finalized = 2,
        }

        #[derive(Debug, Clone)]
        pub struct SubscribeRequest {
            pub accounts: std::collections::HashMap<String, SubscribeRequestFilterAccounts>,
            pub slots: std::collections::HashMap<String, SubscribeRequestFilterSlots>,
            pub transactions: std::collections::HashMap<String, SubscribeRequestFilterTransactions>,
            pub transactions_status: std::collections::HashMap<String, SubscribeRequestFilterTransactions>,
            pub blocks: std::collections::HashMap<String, SubscribeRequestFilterBlocks>,
            pub blocks_meta: std::collections::HashMap<String, SubscribeRequestFilterBlocksMeta>,
            pub entry: std::collections::HashMap<String, SubscribeRequestFilterEntry>,
            pub commitment: Option<i32>,
            pub accounts_data_slice: Vec<SubscribeRequestAccountsDataSlice>,
            pub ping: Option<SubscribeRequestPing>,
        }

        #[derive(Debug, Clone)]
        pub struct SubscribeRequestFilterAccounts {
            pub account: Vec<String>,
            pub owner: Vec<String>,
            pub filters: Vec<SubscribeRequestAccountsDataSlice>,
        }

        #[derive(Debug, Clone)]
        pub struct SubscribeRequestAccountsDataSlice {
            pub offset: u64,
            pub length: u64,
        }

        #[derive(Debug, Clone)]
        pub struct SubscribeRequestFilterTransactions {
            pub vote: Option<bool>,
            pub failed: Option<bool>,
            pub signature: Option<String>,
            pub account_include: Vec<String>,
            pub account_exclude: Vec<String>,
            pub account_required: Vec<String>,
        }

        #[derive(Debug, Clone)]
        pub struct SubscribeRequestFilterSlots {}

        #[derive(Debug, Clone)]
        pub struct SubscribeRequestFilterBlocks {}

        #[derive(Debug, Clone)]
        pub struct SubscribeRequestFilterBlocksMeta {}

        #[derive(Debug, Clone)]
        pub struct SubscribeRequestFilterEntry {}

        #[derive(Debug, Clone)]
        pub struct SubscribeRequestPing {
            pub id: i32,
        }

        #[derive(Debug, Clone)]
        pub struct SubscribeUpdate {
            pub filters: Vec<String>,
            pub update_oneof: Option<UpdateOneof>,
        }

        #[derive(Debug, Clone)]
        pub enum UpdateOneof {
            Account(SubscribeUpdateAccount),
            Slot(SubscribeUpdateSlot),
            Transaction(SubscribeUpdateTransaction),
            Ping(SubscribeUpdatePing),
        }

        #[derive(Debug, Clone)]
        pub struct SubscribeUpdateAccount {
            pub account: Option<Account>,
        }

        #[derive(Debug, Clone)]
        pub struct SubscribeUpdateTransaction {
            pub transaction: Option<Transaction>,
            pub slot: u64,
        }

        #[derive(Debug, Clone)]
        pub struct SubscribeUpdateSlot {
            pub slot: u64,
        }

        #[derive(Debug, Clone)]
        pub struct SubscribeUpdatePing {
            pub id: i32,
        }

        #[derive(Debug, Clone)]
        pub struct Account {
            pub pubkey: String,
            pub lamports: u64,
            pub owner: String,
            pub executable: bool,
            pub rent_epoch: u64,
            pub data: Vec<u8>,
        }

        #[derive(Debug, Clone)]
        pub struct Transaction {
            pub signature: String,
            pub is_vote: bool,
            pub transaction: Vec<u8>,
            pub meta: Option<TransactionStatusMeta>,
            pub message: Option<Message>,
        }

        #[derive(Debug, Clone)]
        pub struct TransactionStatusMeta {
            pub err: Option<String>,
            pub fee: u64,
            pub pre_balances: Vec<u64>,
            pub post_balances: Vec<u64>,
            pub log_messages: Vec<String>,
        }

        #[derive(Debug, Clone)]
        pub struct Message {
            pub account_keys: Vec<String>,
            pub recent_blockhash: String,
        }
    }
}

pub use yellowstone_grpc_proto::prelude::*;

// Mock GRPC client for development
#[derive(Debug)]  // Added Debug here
pub struct GeyserGrpcClient<T> {
    endpoint: String,
    token: Option<String>,
    _interceptor: std::marker::PhantomData<T>,
    sender: Option<mpsc::UnboundedSender<SubscribeRequest>>,
    receiver: Option<mpsc::UnboundedReceiver<Result<SubscribeUpdate, Status>>>,
}

impl<T> GeyserGrpcClient<T> 
where 
    T: tonic::service::Interceptor + Send + 'static,
{
    pub fn build_from_shared(endpoint: String) -> Result<GeyserGrpcClientBuilder, Box<dyn std::error::Error>> {
        Ok(GeyserGrpcClientBuilder {
            endpoint,
            token: None,
        })
    }

    pub async fn subscribe(&mut self) -> Result<(GeyserSink, GeyserStream), Status> {
        let (tx_req, rx_req) = mpsc::unbounded_channel();
        let (tx_resp, rx_resp) = mpsc::unbounded_channel();

        self.sender = Some(tx_req);
        self.receiver = Some(rx_resp);

        // Start mock subscription handler
        let endpoint = self.endpoint.clone();
        tokio::spawn(async move {
            Self::handle_mock_subscription(rx_req, tx_resp, endpoint).await;
        });

        Ok((
            GeyserSink { sender: self.sender.as_ref().unwrap().clone() },
            GeyserStream { receiver: self.receiver.take().unwrap() },
        ))
    }

    pub async fn ping(&mut self, id: i32) -> Result<Response<()>, Status> {
        debug!("Mock ping with id: {}", id);
        Ok(Response::new(()))
    }

    async fn handle_mock_subscription(
        mut request_receiver: mpsc::UnboundedReceiver<SubscribeRequest>,
        response_sender: mpsc::UnboundedSender<Result<SubscribeUpdate, Status>>,
        endpoint: String,
    ) {
        info!("Starting mock subscription handler for endpoint: {}", endpoint);
        
        let mut monitored_accounts = Vec::new();
        
        while let Some(request) = request_receiver.recv().await {
            debug!("Received subscription request with {} account filters", request.accounts.len());
            
            // Extract accounts from request
            for (_, filter) in request.accounts {
                monitored_accounts.extend(filter.account);
            }
            
            // Send periodic mock updates
            let response_sender_clone = response_sender.clone();
            let accounts_clone = monitored_accounts.clone();
            
            tokio::spawn(async move {
                let mut counter = 0;
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
                
                loop {
                    interval.tick().await;
                    counter += 1;
                    
                    // Send mock account updates
                    for (i, account) in accounts_clone.iter().enumerate() {
                        let mock_balance = 1000000 + (counter * 1000) + (i as u64 * 100);
                        
                        let update = SubscribeUpdate {
                            filters: vec!["mock".to_string()],
                            update_oneof: Some(UpdateOneof::Account(SubscribeUpdateAccount {
                                account: Some(Account {
                                    pubkey: account.clone(),
                                    lamports: mock_balance,
                                    owner: "11111111111111111111111111111111".to_string(),
                                    executable: false,
                                    rent_epoch: 0,
                                    data: vec![],
                                }),
                            })),
                        };
                        
                        if response_sender_clone.send(Ok(update)).is_err() {
                            warn!("Failed to send mock update, receiver dropped");
                            return;
                        }
                    }
                    
                    // Occasionally send a mock transaction
                    if counter % 3 == 0 && !accounts_clone.is_empty() {
                        let update = SubscribeUpdate {
                            filters: vec!["mock".to_string()],
                            update_oneof: Some(UpdateOneof::Transaction(SubscribeUpdateTransaction {
                                slot: counter * 1000,
                                transaction: Some(Transaction {
                                    signature: format!("mock_signature_{}", counter),
                                    is_vote: false,
                                    transaction: vec![],
                                    meta: Some(TransactionStatusMeta {
                                        err: None,
                                        fee: 5000,
                                        pre_balances: vec![1000000, 2000000],
                                        post_balances: vec![995000, 2005000],
                                        log_messages: vec!["Program log: Instruction: Transfer".to_string()],
                                    }),
                                    message: Some(Message {
                                        account_keys: accounts_clone[0..2.min(accounts_clone.len())].to_vec(),
                                        recent_blockhash: "mock_blockhash".to_string(),
                                    }),
                                }),
                            })),
                        };
                        
                        if response_sender_clone.send(Ok(update)).is_err() {
                            warn!("Failed to send mock transaction update");
                            return;
                        }
                    }
                }
            });
        }
    }
}

pub struct GeyserGrpcClientBuilder {
    endpoint: String,
    token: Option<String>,
}

impl GeyserGrpcClientBuilder {
    pub fn x_token(mut self, token: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        self.token = token.map(|s| s.to_string());
        Ok(self)
    }

    pub async fn connect<T>(self) -> Result<GeyserGrpcClient<T>, Box<dyn std::error::Error>> 
    where 
        T: tonic::service::Interceptor + Send + 'static,
    {
        info!("Connecting to mock Geyser endpoint: {}", self.endpoint);
        
        Ok(GeyserGrpcClient {
            endpoint: self.endpoint,
            token: self.token,
            _interceptor: std::marker::PhantomData,
            sender: None,
            receiver: None,
        })
    }
}

pub struct GeyserSink {
    sender: mpsc::UnboundedSender<SubscribeRequest>,
}

impl Sink<SubscribeRequest> for GeyserSink {
    type Error = Status;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: SubscribeRequest) -> Result<(), Self::Error> {
        self.sender
            .send(item)
            .map_err(|_| Status::internal("Channel closed"))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

pub struct GeyserStream {
    receiver: mpsc::UnboundedReceiver<Result<SubscribeUpdate, Status>>,
}

impl Stream for GeyserStream {
    type Item = Result<SubscribeUpdate, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}