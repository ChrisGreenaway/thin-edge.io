use crate::{
    error::AgentError,
    state::{AgentStateRepository, State, StateRepository},
};
use json_sm::{
    software_filter_topic, Jsonify, SoftwareError, SoftwareListRequest, SoftwareListResponse,
    SoftwareOperationStatus, SoftwareRequestResponse, SoftwareUpdateRequest,
    SoftwareUpdateResponse,
};
use log::{debug, error, info};
use mqtt_client::{Client, Message, MqttClient, Topic, TopicFilter};
use plugin_sm::plugin_manager::ExternalPlugins;
use std::sync::Arc;
use tedge_config::TEdgeConfigLocation;
use tedge_users::{UserManager, ROOT_USER};

#[derive(Debug)]
pub struct SmAgentConfig {
    pub request_topics: TopicFilter,
    pub request_topic_list: Topic,
    pub request_topic_update: Topic,
    pub response_topic_list: Topic,
    pub response_topic_update: Topic,
    pub errors_topic: Topic,
    pub mqtt_client_config: mqtt_client::Config,
}

impl Default for SmAgentConfig {
    fn default() -> Self {
        let request_topics = TopicFilter::new(software_filter_topic()).expect("Invalid topic");

        let request_topic_list =
            Topic::new(SoftwareListRequest::topic_name()).expect("Invalid topic");

        let request_topic_update =
            Topic::new(SoftwareUpdateRequest::topic_name()).expect("Invalid topic");

        let response_topic_list =
            Topic::new(SoftwareListResponse::topic_name()).expect("Invalid topic");

        let response_topic_update =
            Topic::new(SoftwareUpdateResponse::topic_name()).expect("Invalid topic");

        let errors_topic = Topic::new("tedge/errors").expect("Invalid topic");

        let mqtt_client_config = mqtt_client::Config::default().with_packet_size(10 * 1024 * 1024);

        Self {
            request_topics,
            request_topic_list,
            request_topic_update,
            response_topic_list,
            response_topic_update,
            errors_topic,
            mqtt_client_config,
        }
    }
}

#[derive(Debug)]
pub struct SmAgent {
    config: SmAgentConfig,
    name: String,
    user_manager: UserManager,
    config_location: TEdgeConfigLocation,
    persistance_store: AgentStateRepository,
}

impl SmAgent {
    pub fn new(
        name: &str,
        user_manager: UserManager,
        config_location: TEdgeConfigLocation,
    ) -> Self {
        let persistance_store = AgentStateRepository::new(&config_location);

        Self {
            config: SmAgentConfig::default(),
            name: name.into(),
            user_manager,
            config_location,
            persistance_store,
        }
    }

    pub async fn start(&self) -> Result<(), AgentError> {
        info!("Starting tedge agent");

        let plugins = Arc::new(ExternalPlugins::open(
            &self
                .config_location
                .tedge_config_root_path
                .join("sm-plugins"),
        )?);
        if plugins.empty() {
            error!("Couldn't load plugins from /etc/tedge/sm-plugins");
            return Err(AgentError::NoPlugins);
        }

        let mqtt = Client::connect(self.name.as_str(), &self.config.mqtt_client_config).await?;
        let mut errors = mqtt.subscribe_errors();
        tokio::spawn(async move {
            while let Some(error) = errors.next().await {
                error!("{}", error);
            }
        });

        let () = self.fail_pending_operation(&mqtt).await?;

        // * Maybe it would be nice if mapper/registry responds
        let () = publish_capabilities(&mqtt).await?;

        let () = self.subscribe_and_process(&mqtt, &plugins).await?;

        Ok(())
    }

    async fn subscribe_and_process(
        &self,
        mqtt: &Client,
        plugins: &Arc<ExternalPlugins>,
    ) -> Result<(), AgentError> {
        let mut operations = mqtt.subscribe(self.config.request_topics.clone()).await?;
        while let Some(message) = operations.next().await {
            debug!("Request {:?}", message);

            match &message.topic {
                topic if topic == &self.config.request_topic_list => {
                    let _success = self
                        .handle_software_list_request(
                            mqtt,
                            plugins.clone(),
                            &self.config.response_topic_list,
                            &message,
                        )
                        .await
                        .map_err(|err| {
                            error!("{:?}", err); // log error and discard such that the agent doesn't exit.
                        });
                }

                topic if topic == &self.config.request_topic_update => {
                    let _success = self
                        .handle_software_update_request(
                            mqtt,
                            plugins.clone(),
                            &self.config.response_topic_update,
                            &message,
                        )
                        .await
                        .map_err(|err| {
                            error!("{:?}", err); // log error and discard such that the agent doesn't exit.
                        });
                }

                _ => error!("Unknown operation. Discarded."),
            }
        }

        Ok(())
    }

    async fn handle_software_list_request(
        &self,
        mqtt: &Client,
        plugins: Arc<ExternalPlugins>,
        response_topic: &Topic,
        message: &Message,
    ) -> Result<(), AgentError> {
        let request = match SoftwareListRequest::from_slice(message.payload_trimmed()) {
            Ok(request) => {
                let () = self
                    .persistance_store
                    .store(&State {
                        operation_id: Some(request.id.clone()),
                        operation: Some("list".into()),
                    })
                    .await?;

                request
            }

            Err(error) => {
                debug!("Parsing error: {}", error);
                let _ = mqtt
                    .publish(Message::new(
                        &self.config.errors_topic,
                        format!("{}", error),
                    ))
                    .await?;

                return Err(SoftwareError::ParseError {
                    reason: "Parsing Error".into(),
                }
                .into());
            }
        };
        let executing_response = SoftwareListResponse::new(&request);

        let _ = mqtt
            .publish(Message::new(
                &self.config.response_topic_list,
                executing_response.to_bytes()?,
            ))
            .await?;

        let response = plugins.list(&request).await;

        let _ = mqtt
            .publish(Message::new(response_topic, response.to_bytes()?))
            .await?;

        let _state = self.persistance_store.clear().await?;

        Ok(())
    }

    async fn handle_software_update_request(
        &self,
        mqtt: &Client,
        plugins: Arc<ExternalPlugins>,
        response_topic: &Topic,
        message: &Message,
    ) -> Result<(), AgentError> {
        let request = match SoftwareUpdateRequest::from_slice(message.payload_trimmed()) {
            Ok(request) => {
                let () = self
                    .persistance_store
                    .store(&State {
                        operation_id: Some(request.id.clone()),
                        operation: Some("update".into()),
                    })
                    .await?;

                request
            }

            Err(error) => {
                error!("Parsing error: {}", error);
                let _ = mqtt
                    .publish(Message::new(
                        &self.config.errors_topic,
                        format!("{}", error),
                    ))
                    .await?;

                return Err(SoftwareError::ParseError {
                    reason: "Parsing failed".into(),
                }
                .into());
            }
        };

        let executing_response = SoftwareUpdateResponse::new(&request);
        let _ = mqtt
            .publish(Message::new(response_topic, executing_response.to_bytes()?))
            .await?;

        let response = {
            let _user_guard = self.user_manager.become_user(ROOT_USER)?;
            plugins.process(&request).await
        };

        let _ = mqtt
            .publish(Message::new(response_topic, response.to_bytes()?))
            .await?;

        let _state = self.persistance_store.clear().await?;

        Ok(())
    }

    async fn fail_pending_operation(&self, mqtt: &Client) -> Result<(), AgentError> {
        if let State {
            operation_id: Some(id),
            operation: Some(operation_string),
        } = match self.persistance_store.load().await {
            Ok(state) => state,
            Err(_) => State {
                operation_id: None,
                operation: None,
            },
        } {
            let topic = match operation_string.into() {
                SoftwareOperation::CurrentSoftwareList => &self.config.response_topic_list,

                SoftwareOperation::SoftwareUpdates => &self.config.response_topic_update,

                SoftwareOperation::UnknownOperation => {
                    error!("UnknownOperation in store.");
                    &self.config.errors_topic
                }
            };

            let response = SoftwareRequestResponse::new(&id, SoftwareOperationStatus::Failed);

            let _ = mqtt
                .publish(Message::new(topic, response.to_bytes()?))
                .await?;
        }

        Ok(())
    }
}

async fn publish_capabilities(mqtt: &Client) -> Result<(), AgentError> {
    mqtt.publish(Message::new(&Topic::new("tedge/capabilities/software/list")?, "").retain())
        .await?;

    mqtt.publish(Message::new(&Topic::new("tedge/capabilities/software/update")?, "").retain())
        .await?;

    Ok(())
}

/// Variants of supported software operations.
#[derive(Debug, Clone, PartialEq)]
pub enum SoftwareOperation {
    CurrentSoftwareList,
    SoftwareUpdates,
    UnknownOperation,
}

impl From<String> for SoftwareOperation {
    fn from(s: String) -> Self {
        match s.as_str() {
            r#"list"# => Self::CurrentSoftwareList,
            r#"update"# => Self::SoftwareUpdates,
            _ => Self::UnknownOperation,
        }
    }
}
