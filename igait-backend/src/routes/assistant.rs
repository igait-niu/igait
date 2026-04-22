use std::sync::Arc;

use anyhow::{bail, Context, Result};
use async_openai::{
    config::OpenAIConfig,
    types::{
        AssistantObject, CreateAssistantToolFileSearchResources, CreateAssistantToolResources,
        CreateMessageRequestArgs, CreateRunRequest, CreateThreadRequest, MessageContent,
        MessageContentTextAnnotations, MessageRole, RunStatus, SubmitToolOutputsRunRequest,
        ThreadObject, ToolsOutputs,
    },
    Client,
};
use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::Response,
};
use firebase_auth::FirebaseUser;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use time_util::system_time_from_secs;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest};

use crate::helper::lib::{AppState, AppStatePtr, Job};

/// Represents an update from the assistant to the client.
///
/// # Variants
/// * `Message` - Contains the content of the message sent by the assistant.
/// * `Error` - Contains the content of an error message.
/// * `Waiting` - Contains the content of a waiting message, indicating that the assistant is still processing.
/// * `Jobs` - Contains a list of jobs that the assistant has access to.
///
/// # Notes
/// * Tagged with `type` to allow for easy deserialization.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum AssistantUpdate {
    Message { content: String },
    Error { content: String },
    Waiting { content: String },
    Jobs { content: Vec<Job> },
}
/// Represents the arguments for the `get_last_job` and `get_all_jobs` functions.
///
/// # Fields
/// * `entries` - The number of entries to return.
/// * `start_timestamp` - The UNIX timestamp to start the search from.
/// * `end_timestamp` - The UNIX timestamp to end the search at.
/// * `result_type` - The type of result to return, either `ASD` or `NO ASD`.
#[derive(Deserialize)]
struct SearchJobArguments {
    entries: Option<i64>,
    start_timestamp: Option<i64>,
    end_timestamp: Option<i64>,
    result_type: Option<String>,
}

/// Sends a response to the client over a mutable `WebSocket` connection.
///
/// # Arguments
/// * `app` - The application state containing the OpenAI client and assistant.
/// * `client` - The OpenAI client to use for sending the response.
/// * `thread` - The thread object to send the response in.
/// * `assistant` - The assistant object to use for the response.
/// * `message` - The message to send to the assistant.
/// * `socket` - The mutable WebSocket connection to send the response over.
/// * `user_id` - The user ID of the client sending the message.
///
/// # Fails
/// * If the message fails to be created.
/// * If the run fails to be created.
/// * If the run fails to be polled.
/// * If the run fails to be submitted.
/// * If the message fails to be sent to the client.
/// * If the run fails to be retrieved.
/// * If the run fails to be submitted with tool outputs.
/// * If the run fails to be retrieved after submitting tool outputs.
///
/// # Returns
/// * Nothing
async fn send_response(
    app: &Arc<AppState>,
    client: &Client<OpenAIConfig>,
    thread: &ThreadObject,
    assistant: &AssistantObject,
    message: &str,
    socket: &mut WebSocket,
    user_id: &str,
) -> Result<()> {
    // Create and add the message
    let create_message_request = CreateMessageRequestArgs::default()
        .role(MessageRole::User)
        .content(message)
        .build()?;
    let _message_obj = client
        .threads()
        .messages(&thread.id)
        .create(create_message_request)
        .await?;

    // Create the run
    let create_run_request = CreateRunRequest {
        assistant_id: assistant.id.clone(),
        ..Default::default()
    };
    let mut run = client
        .threads()
        .runs(&thread.id)
        .create(create_run_request)
        .await?;

    // Poll the run object until it is completed
    loop {
        match run.status {
            RunStatus::Completed => {
                let messages = client
                    .threads()
                    .messages(&thread.id)
                    .list(&[("limit", "10")])
                    .await
                    .context("Failed to list messages")?;

                let newest_message_obj = &messages
                    .data
                    .first()
                    .context("No messages found in thread")?;

                let message_contents = &newest_message_obj.content;
                let mut body = Vec::new();
                for message_content in message_contents {
                    match message_content {
                        MessageContent::Text(text) => {
                            let text_data = &text.text;

                            let mut body_to_push = text_data.value.clone();
                            let annotations = &text_data.annotations;
                            for (ind, annotation) in annotations.iter().enumerate() {
                                if let MessageContentTextAnnotations::FileCitation(obj) = annotation
                                {
                                    body_to_push =
                                        body_to_push.replace(&obj.text, &format!("[{}]", ind + 1));
                                }
                            }
                            body.push(body_to_push);
                            if !annotations.is_empty() {
                                body.push(
                                    "\n".to_string()
                                        + "[ From "
                                        + annotations.len().to_string().as_str()
                                        + " Sources ]",
                                );
                            }
                        }
                        MessageContent::ImageFile(_) | MessageContent::ImageUrl(_) => {
                            body.push("[Image]".to_string());
                        }
                        MessageContent::Refusal(refusal) => {
                            body.push(refusal.refusal.to_string());
                        }
                    }
                }
                let event = AssistantUpdate::Message {
                    content: body.join("\n"),
                };
                println!("Sending message to client: {:?}", event);
                socket
                    .send(axum::extract::ws::Message::Text(
                        serde_json::to_string(&event)
                            .context("Failed to serialize 'done thinking' event!")?,
                    ))
                    .await
                    .context("Failed to send message to client! Error: {e:?}")?;

                break;
            }
            RunStatus::Failed | RunStatus::Cancelled | RunStatus::Expired => {
                let bail_text = match run.status {
                    RunStatus::Failed => "Run Failed!",
                    RunStatus::Cancelled => "Run Cancelled!",
                    RunStatus::Expired => "Run Expired!",
                    _ => unreachable!(),
                };

                bail!(bail_text);
            }

            RunStatus::Queued
            | RunStatus::Cancelling
            | RunStatus::InProgress
            | RunStatus::Incomplete => {
                let status_text = match run.status {
                    RunStatus::Queued => "Run queued...",
                    RunStatus::Cancelling => "Cancelling...",
                    RunStatus::InProgress => "In Progress...",
                    RunStatus::Incomplete => "Run incomplete...",
                    RunStatus::RequiresAction => "Run requires action...",
                    _ => unreachable!(),
                };

                let event = AssistantUpdate::Waiting {
                    content: status_text.to_string(),
                };
                socket
                    .send(axum::extract::ws::Message::Text(
                        serde_json::to_string(&event)
                            .context("Failed to serialize 'done thinking' event!")?,
                    ))
                    .await
                    .context("Failed to send message to client! Error: {e:?}")?;
            }

            RunStatus::RequiresAction => {
                let required_action =
                    &(run.required_action).context("No required action found!")?;

                let run_tool_calls = &required_action.submit_tool_outputs.tool_calls;

                let mut returned_tool_outputs = SubmitToolOutputsRunRequest {
                    tool_outputs: vec![],
                    stream: None,
                };
                for run_tool_call in run_tool_calls {
                    let function = &run_tool_call.function;
                    let function_name = &function.name;

                    println!("Run ID requires action: {function_name}");

                    let result = match function_name.as_str() {
                        "get_last_job" => {
                            let jobs = app
                                .db
                                .lock()
                                .await
                                .get_all_jobs(user_id)
                                .await
                                .with_context(|| "Failed to get all jobs for {user_id}!")?;

                            let last_job = jobs
                                .last()
                                .with_context(|| "No jobs found for {user_id}!")?;

                            let last_job_serialized = serde_json::to_string(last_job)
                                .context("Failed to serialize last job!")?;

                            // Send the job over the websocket
                            let event = AssistantUpdate::Jobs {
                                content: vec![last_job.clone()],
                            };
                            println!("Sending jobs to client: {event:#?}");
                            socket
                                .send(axum::extract::ws::Message::Text(
                                    serde_json::to_string(&event)
                                        .context("Failed to serialize 'done thinking' event!")?,
                                ))
                                .await
                                .context("Failed to send message to client! Error: {e:?}")?;

                            last_job_serialized
                        }
                        "get_all_jobs" => {
                            let jobs = app
                                .db
                                .lock()
                                .await
                                .get_all_jobs(user_id)
                                .await
                                .with_context(|| "Failed to get all jobs for {user_id}!")?;

                            let jobs_serialized = serde_json::to_string(&jobs)
                                .context("Failed to serialize last job!")?;

                            // Send the job over the websocket
                            let event = AssistantUpdate::Jobs {
                                content: jobs.clone(),
                            };
                            println!("Sending jobs to client: {event:#?}");
                            socket
                                .send(axum::extract::ws::Message::Text(
                                    serde_json::to_string(&event)
                                        .context("Failed to serialize 'done thinking' event!")?,
                                ))
                                .await
                                .context("Failed to send message to client! Error: {e:?}")?;

                            jobs_serialized
                        }
                        "search_jobs" => {
                            let search_args =
                                serde_json::from_str::<SearchJobArguments>(&function.arguments)
                                    .context("Failed to deserialize search arguments!")?;

                            let mut jobs = app
                                .db
                                .lock()
                                .await
                                .get_all_jobs(user_id)
                                .await
                                .with_context(|| "Failed to search jobs for {user_id}!")?;

                            // Filter the jobs
                            if let Some(entries) = search_args.entries {
                                jobs = jobs.into_iter().take(entries as usize).collect();
                            }
                            if let Some(start_timestamp) = search_args.start_timestamp {
                                jobs = jobs
                                    .into_iter()
                                    .filter_map(|job| {
                                        if job.timestamp
                                            >= system_time_from_secs(serde_json::Value::Number(
                                                start_timestamp.into(),
                                            ))
                                            .ok()?
                                        {
                                            Some(job)
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                            }
                            if let Some(end_timestamp) = search_args.end_timestamp {
                                jobs = jobs
                                    .into_iter()
                                    .filter_map(|job| {
                                        if job.timestamp
                                            <= system_time_from_secs(serde_json::Value::Number(
                                                end_timestamp.into(),
                                            ))
                                            .ok()?
                                        {
                                            Some(job)
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                            }
                            if let Some(result_type) = search_args.result_type {
                                jobs.retain(|job| job.status.description() == result_type);
                            }

                            // Serialized the now-filtered jobs
                            let jobs_serialized = serde_json::to_string(&jobs)
                                .context("Failed to serialize last job!")?;

                            // Send the job over the websocket
                            let event = AssistantUpdate::Jobs {
                                content: jobs.clone(),
                            };
                            println!("Sending jobs to client: {event:#?}");
                            socket
                                .send(axum::extract::ws::Message::Text(
                                    serde_json::to_string(&event)
                                        .context("Failed to serialize 'done thinking' event!")?,
                                ))
                                .await
                                .context("Failed to send message to client! Error: {e:?}")?;

                            jobs_serialized
                        }
                        _ => bail!("Unknown function name: {function_name}"),
                    };

                    returned_tool_outputs.tool_outputs.push(ToolsOutputs {
                        tool_call_id: Some(run_tool_call.id.clone()),
                        output: Some(result),
                    });
                }

                client
                    .threads()
                    .runs(&thread.id)
                    .submit_tool_outputs(&run.id, returned_tool_outputs)
                    .await?;
            }
        }

        // Wait for 1 sec before polling run object again
        sleep(Duration::from_secs(1)).await;

        // Retrieve the run
        run = client.threads().runs(&thread.id).retrieve(&run.id).await?;
    }

    Ok(())
}
/// The entry point for the API-facing assistant route.
///
/// Takes a `WebSocketUpgrade` and upgrades the connection to a WebSocket.
///
/// # Arguments
/// * `app` - The application state containing the OpenAI client and assistant.
/// * `ws` - The WebSocket upgrade request.
///
/// # Returns
/// * A response that upgrades the connection to a WebSocket and handles the socket connection.
pub async fn assistant_proxied_entrypoint(
    State(app): State<AppStatePtr>,
    ws: WebSocketUpgrade,
) -> Response {
    println!("Proxying connection for `assistant` route");

    ws.on_upgrade(move |socket| handle_proxied_socket_helper(app.state, socket))
}
/// A helper function to handle the proxied socket connection.
///
/// # Arguments
/// * `app` - The application state containing the OpenAI client and assistant.
/// * `socket` - The WebSocket connection to handle.
async fn handle_proxied_socket_helper(app: Arc<AppState>, socket: WebSocket) -> () {
    if let Err(e) = handle_proxied_socket(app, socket).await {
        eprintln!("Failed to handle socket! Error: {e:?}");
    }
}
/// Handles the proxied socket connection.
///
/// This function expects to recieve an initial message containing the user's Firebase JWT, which is then passed via the now-proxied WSS.
///
/// # Arguments
/// * `_app` - The application state containing the OpenAI client and assistant.
/// * `socket` - The WebSocket connection to handle.
///
/// # Fails
/// * If the token cannot be received from the client.
/// * If the token cannot be parsed into a valid string.
/// * If the connection to the proxied WebSocket fails.
/// * If the message cannot be sent to the proxied WebSocket.
/// * If the message cannot be received from the proxied WebSocket.
///
/// # Returns
/// * Nothing
async fn handle_proxied_socket(_app: Arc<AppState>, mut socket: WebSocket) -> Result<()> {
    // Get the token from the client
    let token = match socket
        .recv()
        .await
        .context("Failed to receive token from client!")?
    {
        Ok(msg_obj) => match msg_obj {
            axum::extract::ws::Message::Text(text) => text,
            _ => bail!("Expected text message!"),
        },
        Err(e) => bail!("Failed to receive message from client! Error: {e:?}"),
    };

    println!("Received token: {token}");

    let port = std::env::var("PORT").unwrap_or("3000".to_string());
    let mut request = (&format!("ws://localhost:{port}/api/v1/assistant")).into_client_request()?;
    let headers = request.headers_mut();
    headers.insert(
        "Authorization",
        format!("Bearer {token}")
            .parse()
            .context("Failed to parse token header!")?,
    );
    let (mut local_socket, _) = connect_async(request).await?;

    // Get message from client
    'primary_loop: while let Some(msg_result) = tokio::select! {
        msg_res = socket.recv() => msg_res,
        _ = tokio::time::sleep(Duration::from_secs(5 * 60)) => {
            // Tell the client that they are being disconnected
            //  due to inactivity
            let event = AssistantUpdate::Error {
                content: "Connection timed out due to inactivity!".to_string()
            };

            socket.send(
                axum::extract::ws::Message::Text(serde_json::to_string(&event)
                    .context("Failed to serialize 'error' event!")?)
            ).await
                .context("Failed to send message to client! Error: {e:?}")?;

            return Ok(());
        }
    } {
        println!("Received message: {msg_result:?}");
        let msg_obj = if let Ok(msg_obj) = msg_result {
            msg_obj
        } else {
            break;
        };

        let msg = match msg_obj {
            axum::extract::ws::Message::Text(text) => {
                if text == "ping" {
                    println!("Received ping, sending pong...");
                    socket
                        .send(axum::extract::ws::Message::Text("pong".to_string()))
                        .await
                        .context("Couldn't send message!")?;
                    continue 'primary_loop;
                } else {
                    text
                }
            }
            _ => {
                return Ok(());
            }
        };

        println!("Got message on proxied connection, forwarding: {msg}");

        local_socket
            .send(tokio_tungstenite::tungstenite::Message::Text(msg))
            .await
            .context("Couldn't send message!")?;

        println!("Sent!");

        loop {
            while let Some(msg_result) = local_socket.next().await {
                match msg_result.and_then(|msg_obj| msg_obj.into_text()) {
                    Ok(msg) => {
                        socket
                            .send(axum::extract::ws::Message::Text(msg.to_string()))
                            .await
                            .context("Couldn't send message!")?;

                        if msg.starts_with("{\"type\":\"Message\"") {
                            println!("Awaiting new message...");
                            continue 'primary_loop;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to receive message from local socket! Error: {e:?}");

                        break 'primary_loop;
                    }
                }
            }
        }
    }

    Ok(())
}

/// The entry point for the assistant route.
///
/// This route upgrades the connection to a WebSocket and handles the socket connection.
/// Because Firebase Auth is expected to be grabbed via the `Bearer` header (which is in theory supported by WSS, considering the initial GET request involved with establishing a websocket!), it must be proxied to include the header.
/// This is done because sending a WSS request with a `Bearer` header is not supported by the WSS standard for JavaScript clients.
///
/// # Arguments
/// * `current_user` - The authenticated Firebase user.
/// * `app` - The application state containing the OpenAI client and assistant.
/// * `ws` - The WebSocket upgrade request.
///
/// # Returns
/// * A response that upgrades the connection to a WebSocket and handles the socket connection.
pub async fn assistant_entrypoint(
    current_user: FirebaseUser,
    State(app): State<AppStatePtr>,
    ws: WebSocketUpgrade,
) -> Response {
    println!("Upgrading WS connection...");
    ws.on_upgrade(move |socket| handle_socket_helper(app.state, socket, current_user))
}
/// A helper function to handle the socket connection.
///
/// # Arguments
/// * `app` - The application state containing the OpenAI client and assistant.
/// * `socket` - The WebSocket connection to handle.
/// * `current_user` - The authenticated Firebase user.
async fn handle_socket_helper(
    app: Arc<AppState>,
    socket: WebSocket,
    current_user: FirebaseUser,
) -> () {
    if let Err(e) = handle_socket(app, socket, current_user).await {
        eprintln!("Failed to handle socket! Error: {e:?}");
    }
}
/// Handles the socket connection for the assistant route.
///
/// This function expects the user to be authenticated via Firebase Auth, and will create a new OpenAI thread for the user.
///
/// # Arguments
/// * `app` - The application state containing the OpenAI client and assistant.
/// * `socket` - The WebSocket connection to handle.
/// * `current_user` - The authenticated Firebase user.
///
/// # Fails
/// * If the OpenAI vector store ID cannot be found in the environment variables.
/// * If the thread creation fails.
/// * If the message cannot be sent to the client.
/// * If the run fails to be created or polled.
/// * If the run fails to be submitted with tool outputs.
/// * If the run fails to be retrieved after submitting tool outputs.
///
/// # Returns
/// * Nothing
async fn handle_socket(
    app: Arc<AppState>,
    mut socket: WebSocket,
    current_user: FirebaseUser,
) -> Result<()> {
    let id = &current_user.user_id;

    println!("User ID '{id}' connected to assistant!");

    let vector_store_id = std::env::var("OPENAI_VECTOR_STORE_ID")
        .context("Couldn't find the OpenAI vector store ID!")?;

    let create_thread_request = CreateThreadRequest {
        messages: None,
        tool_resources: Some(CreateAssistantToolResources {
            code_interpreter: None,
            file_search: Some(CreateAssistantToolFileSearchResources {
                vector_store_ids: Some(vec![vector_store_id]),
                ..Default::default()
            }),
        }),
        ..Default::default()
    };
    let thread = app
        .openai_client
        .threads()
        .create(create_thread_request)
        .await?;

    while let Some(msg_result) = socket.recv().await {
        println!("Received message: {msg_result:?}");
        let msg_obj = if let Ok(msg_obj) = msg_result {
            msg_obj
        } else {
            break;
        };

        let msg = match msg_obj {
            axum::extract::ws::Message::Text(text) => text,
            _ => {
                return Ok(());
            }
        };

        let assistant = match &app.openai_assistant {
            Some(a) => a,
            None => {
                let event = AssistantUpdate::Error {
                    content: "AI Assistant is not configured. Please set OPENAI_ASSISTANT_ID."
                        .to_string(),
                };
                socket
                    .send(axum::extract::ws::Message::Text(serde_json::to_string(
                        &event,
                    )?))
                    .await?;
                return Ok(());
            }
        };

        if let Err(e) = send_response(
            &app,
            &app.openai_client,
            &thread,
            assistant,
            &msg,
            &mut socket,
            id,
        )
        .await
        {
            let event = AssistantUpdate::Error {
                content: e.to_string(),
            };
            socket
                .send(axum::extract::ws::Message::Text(
                    serde_json::to_string(&event).context("Failed to serialize 'error' event!")?,
                ))
                .await
                .context("Failed to send message to client! Error: {e:?}")?;
        }
    }

    // Close the thread
    app.openai_client.threads().delete(&thread.id).await?;
    println!("Thread closed!");

    Ok(())
}
