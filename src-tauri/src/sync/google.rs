//! The Google Calendar REST client.
//!
//! Only the calls the sync needs are here. The client is written against a
//! trait so the tests can run without a network.

use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;

use crate::sync::payload::{CalendarPayload, EventPayload};

/// A boxed future — the client is used through a trait object.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The namespace checkd puts its calendars in.
///
/// adventure_list uses `location: "adventure_list"`; checkd has its own, so
/// the two apps never touch each other's calendars. The calendars are hidden
/// from the user's calendar list.
pub const NAMESPACE: &str = "checkd";

const API_BASE: &str = "https://www.googleapis.com/calendar/v3";

/// A calendar of the connected account.
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteCalendar {
    pub id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    /// The namespace — `checkd` for the calendars that belong to this app.
    pub location: Option<String>,
    /// Google leaves the flag out when the calendar is not hidden.
    #[serde(default)]
    pub hidden: bool,
    /// Set by `calendarList.list` with `showDeleted=true` when removed.
    #[serde(default)]
    pub deleted: bool,
}

/// An event of a checkd calendar.
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteEvent {
    pub id: String,
    /// The id of the local task this event mirrors.
    #[serde(rename = "iCalUID")]
    pub ical_uid: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    /// `cancelled` means the event was deleted on the Google side.
    pub status: Option<String>,
    /// When the event was last changed on the Google side.
    pub updated: Option<String>,
}

/// The result of listing events.
#[derive(Debug, Clone)]
pub struct EventList {
    pub events: Vec<RemoteEvent>,
    pub next_sync_token: Option<String>,
}

/// The result of listing calendars.
#[derive(Debug, Clone)]
pub struct CalendarList {
    pub calendars: Vec<RemoteCalendar>,
    pub next_sync_token: Option<String>,
}

/// The Calendar calls the sync needs.
pub trait GoogleClient: Send + Sync + std::fmt::Debug {
    /// The calendars of the account that belong to checkd.
    fn list_calendars<'a>(
        &'a self,
        access_token: &'a str,
    ) -> BoxFuture<'a, Result<Vec<RemoteCalendar>, String>>;

    /// The calendars of the account that belong to checkd with sync token support.
    fn list_calendars_delta<'a>(
        &'a self,
        access_token: &'a str,
        sync_token: Option<&'a str>,
    ) -> BoxFuture<'a, Result<CalendarList, String>>;

    /// The events of one calendar.
    fn list_events<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<RemoteEvent>, String>>;

    /// The events of one calendar with sync token support.
    fn list_events_delta<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
        sync_token: Option<&'a str>,
    ) -> BoxFuture<'a, Result<EventList, String>>;

    /// Creates a secondary calendar for a list and hides it from the user's normal calendar list.
    fn create_calendar<'a>(
        &'a self,
        access_token: &'a str,
        calendar: &'a CalendarPayload,
    ) -> BoxFuture<'a, Result<RemoteCalendar, String>>;

    /// Updates the metadata of a calendar.
    fn update_calendar<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
        calendar: &'a CalendarPayload,
    ) -> BoxFuture<'a, Result<RemoteCalendar, String>>;

    /// Deletes a secondary calendar.
    fn delete_calendar<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
    ) -> BoxFuture<'a, Result<(), String>>;

    /// Hides a calendar in the user's `calendarList`.
    fn hide_calendar<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
    ) -> BoxFuture<'a, Result<(), String>>;

    /// Creates an event in a calendar.
    fn create_event<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
        event: &'a EventPayload,
    ) -> BoxFuture<'a, Result<RemoteEvent, String>>;

    /// Updates an event in a calendar.
    fn update_event<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
        event_id: &'a str,
        event: &'a EventPayload,
    ) -> BoxFuture<'a, Result<RemoteEvent, String>>;

    /// Deletes an event from a calendar.
    fn delete_event<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
        event_id: &'a str,
    ) -> BoxFuture<'a, Result<(), String>>;
}

/// The real client — talks to the Calendar API over HTTPS.
#[derive(Debug)]
pub struct GoogleApi {
    client: reqwest::Client,
    base: String,
}

impl GoogleApi {
    pub fn new() -> Self {
        GoogleApi {
            client: reqwest::Client::new(),
            base: API_BASE.to_string(),
        }
    }

    /// Sends a GET and reads the JSON answer.
    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        access_token: &str,
        url: &str,
    ) -> Result<T, String> {
        let response = self
            .client
            .get(url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status();
        let body = response.text().await.map_err(|e| e.to_string())?;

        if !status.is_success() {
            return Err(describe_error(&body, status.as_u16()));
        }

        serde_json::from_str(&body).map_err(|e| e.to_string())
    }

    /// Sends a POST with a JSON body and reads the JSON answer.
    async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        access_token: &str,
        url: &str,
        payload: &B,
    ) -> Result<T, String> {
        let response = self
            .client
            .post(url)
            .bearer_auth(access_token)
            .json(payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status();
        let body = response.text().await.map_err(|e| e.to_string())?;

        if !status.is_success() {
            return Err(describe_error(&body, status.as_u16()));
        }

        serde_json::from_str(&body).map_err(|e| e.to_string())
    }

    /// Sends a PUT with a JSON body and reads the JSON answer.
    async fn put_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        access_token: &str,
        url: &str,
        payload: &B,
    ) -> Result<T, String> {
        let response = self
            .client
            .put(url)
            .bearer_auth(access_token)
            .json(payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status();
        let body = response.text().await.map_err(|e| e.to_string())?;

        if !status.is_success() {
            return Err(describe_error(&body, status.as_u16()));
        }

        serde_json::from_str(&body).map_err(|e| e.to_string())
    }

    /// Sends a PATCH with a JSON body and ignores the response body on success.
    async fn patch_json<B: serde::Serialize>(
        &self,
        access_token: &str,
        url: &str,
        payload: &B,
    ) -> Result<(), String> {
        let response = self
            .client
            .patch(url)
            .bearer_auth(access_token)
            .json(payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status();
        let body = response.text().await.map_err(|e| e.to_string())?;

        if !status.is_success() {
            return Err(describe_error(&body, status.as_u16()));
        }

        Ok(())
    }

    /// Sends a DELETE request.
    async fn delete_req(&self, access_token: &str, url: &str) -> Result<(), String> {
        let response = self
            .client
            .delete(url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status();
        let body = response.text().await.map_err(|e| e.to_string())?;

        if !status.is_success() {
            return Err(describe_error(&body, status.as_u16()));
        }

        Ok(())
    }
}

impl Default for GoogleApi {
    fn default() -> Self {
        GoogleApi::new()
    }
}

impl GoogleClient for GoogleApi {
    fn list_calendars<'a>(
        &'a self,
        access_token: &'a str,
    ) -> BoxFuture<'a, Result<Vec<RemoteCalendar>, String>> {
        Box::pin(async move {
            let list = self.list_calendars_delta(access_token, None).await?;
            Ok(list.calendars)
        })
    }

    fn list_calendars_delta<'a>(
        &'a self,
        access_token: &'a str,
        sync_token: Option<&'a str>,
    ) -> BoxFuture<'a, Result<CalendarList, String>> {
        Box::pin(async move {
            let mut all_items: Vec<RemoteCalendar> = Vec::new();
            let mut page_token: Option<String> = None;
            let next_sync_token_result: Option<String>;

            loop {
                #[derive(Deserialize)]
                struct Answer {
                    #[serde(default)]
                    items: Vec<RemoteCalendar>,
                    #[serde(rename = "nextPageToken")]
                    next_page_token: Option<String>,
                    #[serde(rename = "nextSyncToken")]
                    next_sync_token: Option<String>,
                }

                let mut url = if let Some(token) = sync_token {
                    format!(
                        "{}/users/me/calendarList?fields=items(id,summary,description,location,hidden,deleted),nextPageToken,nextSyncToken&showHidden=true&showDeleted=true&syncToken={}",
                        self.base,
                        urlencode(token)
                    )
                } else {
                    format!(
                        "{}/users/me/calendarList?fields=items(id,summary,description,location,hidden,deleted),nextPageToken,nextSyncToken&showHidden=true",
                        self.base
                    )
                };
                if let Some(ref token) = page_token {
                    url.push_str("&pageToken=");
                    url.push_str(&urlencode(token));
                }
                let answer: Answer = match self.get_json(access_token, &url).await {
                    Ok(answer) => answer,
                    Err(err) => {
                        if err.contains("410") || err.contains("Gone") {
                            if sync_token.is_some() {
                                return self.list_calendars_delta(access_token, None).await;
                            }
                        }
                        return Err(err);
                    }
                };
                all_items.extend(answer.items);

                if answer.next_page_token.is_some() {
                    page_token = answer.next_page_token;
                } else {
                    next_sync_token_result = answer.next_sync_token.clone();
                    break;
                }
            }

            Ok(CalendarList {
                calendars: all_items
                    .into_iter()
                    .filter(|calendar| {
                        calendar.deleted || calendar.location.as_deref() == Some(NAMESPACE)
                    })
                    .collect(),
                next_sync_token: next_sync_token_result,
            })
        })
    }

    fn list_events<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<RemoteEvent>, String>> {
        Box::pin(async move {
            let list = self.list_events_delta(access_token, calendar_id, None).await?;
            Ok(list.events)
        })
    }

    fn list_events_delta<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
        sync_token: Option<&'a str>,
    ) -> BoxFuture<'a, Result<EventList, String>> {
        Box::pin(async move {
            let mut all_items: Vec<RemoteEvent> = Vec::new();
            let mut page_token: Option<String> = None;
            let next_sync_token_result: Option<String>;

            loop {
                #[derive(Deserialize)]
                struct Answer {
                    #[serde(default)]
                    items: Vec<RemoteEvent>,
                    #[serde(rename = "nextPageToken")]
                    next_page_token: Option<String>,
                    #[serde(rename = "nextSyncToken")]
                    next_sync_token: Option<String>,
                }

                let mut url = if let Some(token) = sync_token {
                    format!(
                        "{}/calendars/{}/events?fields=items(id,iCalUID,summary,description,status,updated),nextPageToken,nextSyncToken&showDeleted=true&singleEvents=false&syncToken={}",
                        self.base,
                        urlencode(calendar_id),
                        urlencode(token)
                    )
                } else {
                    format!(
                        "{}/calendars/{}/events?fields=items(id,iCalUID,summary,description,status,updated),nextPageToken,nextSyncToken&maxResults=250&showDeleted=true&singleEvents=false",
                        self.base,
                        urlencode(calendar_id)
                    )
                };
                if let Some(ref token) = page_token {
                    url.push_str("&pageToken=");
                    url.push_str(&urlencode(token));
                }
                let answer: Answer = match self.get_json(access_token, &url).await {
                    Ok(answer) => answer,
                    Err(err) => {
                        if err.contains("410") || err.contains("Gone") {
                            if sync_token.is_some() {
                                return self.list_events_delta(access_token, calendar_id, None).await;
                            }
                        }
                        return Err(err);
                    }
                };
                all_items.extend(answer.items);

                if answer.next_page_token.is_some() {
                    page_token = answer.next_page_token;
                } else {
                    next_sync_token_result = answer.next_sync_token.clone();
                    break;
                }
            }
            Ok(EventList {
                events: all_items,
                next_sync_token: next_sync_token_result,
            })
        })
    }

    fn create_calendar<'a>(
        &'a self,
        access_token: &'a str,
        calendar: &'a CalendarPayload,
    ) -> BoxFuture<'a, Result<RemoteCalendar, String>> {
        Box::pin(async move {
            let url = format!("{}/calendars", self.base);
            let created: RemoteCalendar = self.post_json(access_token, &url, calendar).await?;
            let _ = self.hide_calendar(access_token, &created.id).await;
            Ok(created)
        })
    }

    fn update_calendar<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
        calendar: &'a CalendarPayload,
    ) -> BoxFuture<'a, Result<RemoteCalendar, String>> {
        Box::pin(async move {
            let url = format!("{}/calendars/{}", self.base, urlencode(calendar_id));
            self.put_json(access_token, &url, calendar).await
        })
    }

    fn delete_calendar<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let url = format!("{}/calendars/{}", self.base, urlencode(calendar_id));
            self.delete_req(access_token, &url).await
        })
    }

    fn hide_calendar<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            #[derive(serde::Serialize)]
            struct Patch {
                hidden: bool,
            }
            let url = format!(
                "{}/users/me/calendarList/{}",
                self.base,
                urlencode(calendar_id)
            );
            self.patch_json(access_token, &url, &Patch { hidden: true })
                .await
        })
    }

    fn create_event<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
        event: &'a EventPayload,
    ) -> BoxFuture<'a, Result<RemoteEvent, String>> {
        Box::pin(async move {
            let url = format!("{}/calendars/{}/events", self.base, urlencode(calendar_id));
            self.post_json(access_token, &url, event).await
        })
    }

    fn update_event<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
        event_id: &'a str,
        event: &'a EventPayload,
    ) -> BoxFuture<'a, Result<RemoteEvent, String>> {
        Box::pin(async move {
            let url = format!(
                "{}/calendars/{}/events/{}",
                self.base,
                urlencode(calendar_id),
                urlencode(event_id)
            );
            self.put_json(access_token, &url, event).await
        })
    }

    fn delete_event<'a>(
        &'a self,
        access_token: &'a str,
        calendar_id: &'a str,
        event_id: &'a str,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let url = format!(
                "{}/calendars/{}/events/{}",
                self.base,
                urlencode(calendar_id),
                urlencode(event_id)
            );
            self.delete_req(access_token, &url).await
        })
    }
}

/// Percent-encodes a path segment.
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Turns Google's error answer into something a person can read.
fn describe_error(body: &str, code: u16) -> String {
    let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or_default();

    if let Some(error) = parsed.get("error") {
        // The Calendar API is not enabled in the project the OAuth client
        // belongs to — the most common problem by far, and it has its own
        // answer.
        if api_is_disabled(error) {
            let link = error
                .get("message")
                .and_then(|message| message.as_str())
                .and_then(link_in)
                .map(|link| format!("enable it at {link} and try again"))
                .unwrap_or_else(|| {
                    "enable it in the Google Cloud console and try again".to_string()
                });

            return format!("The Google Calendar API is not enabled for this project yet — {link}.");
        }

        if let Some(message) = error.get("message").and_then(|message| message.as_str()) {
            return message.to_string();
        }
    }

    match code {
        401 => "Google rejected the access token — please connect again.".to_string(),
        403 => "The account is not allowed to use this calendar.".to_string(),
        404 => "The calendar is gone on the Google side.".to_string(),
        429 => "Too many requests — Google asked us to slow down.".to_string(),
        _ => format!("Google refused the request ({code})."),
    }
}

/// Whether the answer says that the Calendar API is not enabled in the
/// project the OAuth client belongs to.
fn api_is_disabled(error: &serde_json::Value) -> bool {
    let message = error
        .get("message")
        .and_then(|message| message.as_str())
        .unwrap_or_default();

    if message.contains("has not been used in project") {
        return true;
    }

    error
        .get("details")
        .and_then(|details| details.as_array())
        .map(|details| {
            details.iter().any(|detail| {
                detail.get("reason").and_then(|reason| reason.as_str()) == Some("SERVICE_DISABLED")
            })
        })
        .unwrap_or(false)
}

/// The first link in a message.
fn link_in(message: &str) -> Option<&str> {
    let start = message.find("https://")?;
    let end = message[start..]
        .find(char::is_whitespace)
        .map(|offset| start + offset)
        .unwrap_or(message.len());

    Some(message[start..end].trim_end_matches(['.', ',', ')']))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_checkd_calendars_are_kept() {
        let items = r#"{
            "items": [
                {"id": "a", "summary": "Work", "location": "adventure_list"},
                {"id": "b", "summary": "Groceries", "location": "checkd"},
                {"id": "c", "summary": "No location"}
            ]
        }"#;

        #[derive(Deserialize)]
        struct Answer {
            items: Vec<RemoteCalendar>,
        }

        let answer: Answer = serde_json::from_str(items).expect("parse");
        let mine: Vec<&RemoteCalendar> = answer
            .items
            .iter()
            .filter(|calendar| calendar.location.as_deref() == Some(NAMESPACE))
            .collect();

        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].id, "b");
    }

    #[test]
    fn an_event_is_read_from_the_google_shape() {
        let raw = r#"{
            "id": "evt1",
            "iCalUID": "task-1",
            "summary": "Milk",
            "description": "{}",
            "status": "confirmed",
            "updated": "2026-10-22T10:00:00.000Z"
        }"#;

        let event: RemoteEvent = serde_json::from_str(raw).expect("parse");

        assert_eq!(event.id, "evt1");
        assert_eq!(event.ical_uid.as_deref(), Some("task-1"));
        assert_eq!(event.summary.as_deref(), Some("Milk"));
        assert_eq!(event.status.as_deref(), Some("confirmed"));
    }

    /// The answer Google sends when the Calendar API is not enabled in the
    /// project — the most common problem, and it deserves its own answer.
    #[test]
    fn a_disabled_api_gets_a_short_answer() {
        let body = r#"{
            "error": {
                "code": 403,
                "message": "Google Calendar API has not been used in project 871011474160 before or it is disabled. Enable it by visiting https://console.developers.google.com/apis/api/calendar-json.googleapis.com/overview?project=871011474160 then retry. If you enabled this API recently, wait a few minutes for the action to propagate to our systems and retry.",
                "status": "PERMISSION_DENIED",
                "details": [
                    {
                        "@type": "type.googleapis.com/google.rpc.ErrorInfo",
                        "reason": "SERVICE_DISABLED",
                        "domain": "googleapis.com"
                    }
                ]
            }
        }"#;

        let message = describe_error(body, 403);

        assert!(message.starts_with("The Google Calendar API is not enabled"));
        assert!(message.contains("https://console.developers.google.com/apis/api/calendar-json.googleapis.com/overview?project=871011474160"));
        // The wall of text from Google does not reach the user.
        assert!(!message.contains("wait a few minutes"));
    }

    #[test]
    fn other_errors_keep_their_message() {
        let body = r#"{
            "error": {
                "code": 404,
                "message": "Not Found",
                "status": "NOT_FOUND"
            }
        }"#;

        assert_eq!(describe_error(body, 404), "Not Found");
    }
}
