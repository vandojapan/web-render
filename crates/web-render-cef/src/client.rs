use crate::convert::ConversionError;
use crate::protocol;
use crate::types::{InitializeResponse, Notification, RenderRequest, RenderResponse};
use tonic::IntoRequest;

type LibServerClient =
    protocol::libserver::lib_server_client::LibServerClient<tonic::transport::Channel>;

#[derive(Debug, Clone)]
pub struct Client {
    inner: LibServerClient,
}

impl Client {
    pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        let inner = LibServerClient::connect(
            tonic::transport::Endpoint::new(dst)?
                .connect_timeout(std::time::Duration::from_secs(1)),
        )
        .await?
        .accept_compressed(tonic::codec::CompressionEncoding::Gzip)
        .max_decoding_message_size(usize::MAX);
        Ok(Self { inner })
    }

    pub async fn initialize(
        &mut self,
        root_path: impl Into<String>,
        timeout: Option<std::time::Duration>,
    ) -> Result<InitializeResponse, tonic::Status> {
        let request = protocol::libserver::InitializeRequest {
            root_path: root_path.into(),
        };
        let mut request = request.into_request();
        if let Some(timeout) = timeout {
            request.set_timeout(timeout);
        }
        let response = self.inner.initialize(request).await?.into_inner();
        InitializeResponse::try_from(response).map_err(ConversionError::into_status)
    }

    pub async fn batch_render(
        &mut self,
        requests: Vec<RenderRequest>,
    ) -> Result<Vec<RenderResponse>, tonic::Status> {
        let mut render_requests = Vec::with_capacity(requests.len());
        let mut nonces = Vec::with_capacity(requests.len());
        for request in requests {
            let nonce = request.render_nonce;
            if nonce <= 0 || nonces.contains(&nonce) {
                return Err(tonic::Status::invalid_argument(
                    "Request IDs must be unique and positive",
                ));
            }
            render_requests.push(request.into_proto());
            nonces.push(nonce);
        }
        let request = protocol::common::BatchRenderRequest { render_requests };
        let response = self.inner.batch_render(request).await?.into_inner();
        validate_responses(&nonces, response.render_responses)
    }

    pub async fn purge_cache(&mut self) -> Result<(), tonic::Status> {
        self.inner.purge_cache(protocol::common::Void {}).await?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), tonic::Status> {
        self.inner.shutdown(protocol::common::Void {}).await?;
        Ok(())
    }

    pub async fn subscribe_notifications(&mut self) -> Result<NotificationStream, tonic::Status> {
        let response = self
            .inner
            .subscribe_notifications(protocol::common::Void {})
            .await?
            .into_inner();
        Ok(NotificationStream { inner: response })
    }
}

fn validate_responses(
    expected: &[i32],
    received: Vec<protocol::libserver::RenderResponse>,
) -> Result<Vec<RenderResponse>, tonic::Status> {
    let mut remaining: std::collections::HashSet<i32> = expected.iter().copied().collect();
    let mut result = Vec::with_capacity(received.len());
    for response in received {
        if !remaining.remove(&response.render_nonce) {
            return Err(tonic::Status::data_loss(format!(
                "Unexpected or duplicate response ID {}",
                response.render_nonce
            )));
        }
        result.push(RenderResponse::try_from(response).map_err(ConversionError::into_status)?);
    }
    if !remaining.is_empty() {
        return Err(tonic::Status::data_loss(format!(
            "Missing response IDs: {remaining:?}"
        )));
    }
    Ok(result)
}

pub struct NotificationStream {
    inner: tonic::Streaming<protocol::libserver::Notification>,
}

impl NotificationStream {
    pub async fn message(&mut self) -> Result<Option<Notification>, tonic::Status> {
        match self.inner.message().await? {
            Some(notification) => Notification::try_from(notification)
                .map(Some)
                .map_err(ConversionError::into_status),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn response(id: i32) -> protocol::libserver::RenderResponse {
        protocol::libserver::RenderResponse {
            render_nonce: id,
            response: Some(
                protocol::libserver::render_response::Response::ErrorMessage("scene error".into()),
            ),
        }
    }
    #[test]
    fn accepts_reordering_and_keeps_explicit_ids() {
        let responses = validate_responses(&[1, 2], vec![response(2), response(1)]).unwrap();
        assert_eq!(
            responses.iter().map(|r| r.render_nonce).collect::<Vec<_>>(),
            [2, 1]
        );
    }
    #[test]
    fn rejects_missing_duplicate_extra_stale_and_unset_responses() {
        for received in [
            vec![response(1)],
            vec![response(1), response(1)],
            vec![response(1), response(2), response(3)],
            vec![response(0), response(2)],
            vec![
                protocol::libserver::RenderResponse {
                    render_nonce: 1,
                    response: None,
                },
                response(2),
            ],
        ] {
            assert!(validate_responses(&[1, 2], received).is_err());
        }
        assert!(validate_responses(&[], vec![]).is_ok());
    }
}
