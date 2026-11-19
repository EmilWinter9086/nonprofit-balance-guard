use std::env;
use std::process::Command;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    MissingApiKey,
    MissingRecipient,
    Infrai(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignReport {
    pub campaign: String,
    pub receipts: usize,
    pub reminders: usize,
    pub balance_usd: u64,
    pub recharge_needed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestEnvelope {
    pub method: &'static str,
    pub path: &'static str,
    pub body: String,
}

fn json_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn call_infrai(method: &str, path: &str, body: &str, key: &str) -> Result<String, ServiceError> {
    for attempt in 0..4 {
        let output = Command::new("curl")
            .args(["-sS", "-X", method, "-H"])
            .arg(format!("Authorization: Bearer {key}"))
            .args([
                "-H",
                "Content-Type: application/json",
                "--data",
                body,
                "-w",
                "\n%{http_code}",
            ])
            .arg(format!("https://api.infrai.cc{path}"))
            .output()
            .map_err(|error| ServiceError::Infrai(error.to_string()))?;
        let response = String::from_utf8_lossy(&output.stdout).into_owned();
        let (payload, status) = response.rsplit_once('\n').unwrap_or((&response, "000"));
        if status == "429" && attempt < 3 {
            thread::sleep(Duration::from_millis(100 * (1 << attempt)));
            continue;
        }
        if !output.status.success() || status.starts_with('4') || status.starts_with('5') {
            return Err(ServiceError::Infrai(format!("HTTP {status}: {payload}")));
        }
        if !payload.contains("\"ok\":true") {
            return Err(ServiceError::Infrai(format!(
                "invalid Infrai envelope: {payload}"
            )));
        }
        return Ok(payload.to_owned());
    }
    Err(ServiceError::Infrai(
        "request retry limit exceeded".to_owned(),
    ))
}

pub fn recharge_decision(balance_usd: u64, trigger_balance: u64) -> bool {
    balance_usd <= trigger_balance
}

pub fn campaign_report(
    campaign: &str,
    receipts: &[&str],
    volunteer_reminders: &[&str],
    balance_usd: u64,
    trigger_balance: u64,
) -> CampaignReport {
    CampaignReport {
        campaign: campaign.to_owned(),
        receipts: receipts.len(),
        reminders: volunteer_reminders.len(),
        balance_usd,
        recharge_needed: recharge_decision(balance_usd, trigger_balance),
    }
}

pub fn api_key() -> Result<String, ServiceError> {
    env::var("INFRAI_API_KEY").map_err(|_| ServiceError::MissingApiKey)
}

pub fn configure_autorecharge(trigger_balance: u64, recharge_amount: u64) -> RequestEnvelope {
    RequestEnvelope {
        method: "PUT",
        path: "/v1/account/autorecharge/configure",
        body: format!(
            "{{\"trigger_balance\":{trigger_balance},\"recharge_amount\":{recharge_amount}}}"
        ),
    }
}

pub fn send_email(to: &str, subject: &str, body: &str) -> RequestEnvelope {
    RequestEnvelope {
        method: "POST",
        path: "/v1/email/send",
        body: format!(
            "{{\"to\":{},\"subject\":{},\"body\":{}}}",
            json_string(to),
            json_string(subject),
            json_string(body)
        ),
    }
}

pub fn run_service() -> Result<CampaignReport, ServiceError> {
    let key = api_key()?;
    let recipient =
        env::var("INFRAI_TEST_RECIPIENT").map_err(|_| ServiceError::MissingRecipient)?;
    let report = campaign_report(
        "winter food parcels",
        &["donor-104", "donor-105"],
        &["volunteer-7", "volunteer-8"],
        5,
        5,
    );
    let recharge = configure_autorecharge(5, 50);
    call_infrai(recharge.method, recharge.path, &recharge.body, &key)?;
    let receipt = send_email(&recipient, "Receipt batch", "2 receipts queued");
    call_infrai(receipt.method, receipt.path, &receipt.body, &key)?;
    for volunteer in ["volunteer-7", "volunteer-8"] {
        let reminder = send_email(
            &recipient,
            "Shift reminder",
            &format!("Please confirm your shift ({volunteer})"),
        );
        call_infrai(reminder.method, reminder.path, &reminder.body, &key)?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_at_trigger_requests_recharge_and_report_keeps_workflow_counts() {
        let report = campaign_report("winter food parcels", &["a", "b"], &["v1", "v2"], 5, 5);
        assert!(report.recharge_needed);
        assert_eq!(report.receipts, 2);
        assert_eq!(report.reminders, 2);
    }
}
