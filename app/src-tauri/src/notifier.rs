use crate::domain::TaskStatus;

pub struct OutboundNotification {
    pub status: TaskStatus,
    pub title: String,
}

pub fn notification_text(notification: &OutboundNotification) -> String {
    let status_text = match notification.status {
        TaskStatus::NeedsPermission => "需要权限",
        TaskStatus::NeedsConfirmation => "需要确认",
        TaskStatus::Completed => "已完成",
        TaskStatus::Interrupted => "异常中断",
        TaskStatus::Executing => "执行中",
    };

    format!("{}：{}", status_text, notification.title)
}

pub struct BarkNotifier {
    endpoint_url: String,
    client: reqwest::Client,
}

impl BarkNotifier {
    pub fn new(endpoint_url: impl Into<String>) -> Self {
        Self {
            endpoint_url: endpoint_url.into(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn send(&self, notification: &OutboundNotification) -> Result<(), String> {
        let url = format!(
            "{}/{}",
            self.endpoint_url.trim_end_matches('/'),
            notification_text(notification)
        );
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| err.to_string())?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Bark request failed with status {}",
                response.status()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_permission_notification() {
        let notification = OutboundNotification {
            status: TaskStatus::NeedsPermission,
            title: "重构登录页权限判断".to_string(),
        };

        assert_eq!(
            notification_text(&notification),
            "需要权限：重构登录页权限判断"
        );
    }

    #[test]
    fn formats_interrupted_notification() {
        let notification = OutboundNotification {
            status: TaskStatus::Interrupted,
            title: "接入支付回调测试".to_string(),
        };

        assert_eq!(
            notification_text(&notification),
            "异常中断：接入支付回调测试"
        );
    }
}
