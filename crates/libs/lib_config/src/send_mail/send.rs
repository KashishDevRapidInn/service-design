use errors::CustomError;
use reqwest::Client;
use serde::Serialize; 
use crate::config::configuration;

#[derive(Serialize)]
struct MailgunPayload {
    from: String,
    to: String,
    subject: String,
    text: String,
}

pub async fn send_email(to: &str, mail_token: String) -> Result<(), CustomError> {
    let config = configuration::Settings::new().expect("Failed to load configurations");

    let domain = config.mail.mail_domain; 
    let api_key = config.mail.api_key;  

    let verification_link = format!(
        "{}/user/api/v1/users/verify-email?token={}",
        config.mail.mail_url,
        mail_token
    );
    let subject = "Please verify your email address";

    let body= &format!("Click the link to verify your email: {}", verification_link);

    let client = Client::new();
    let url = format!("https://api.mailgun.net/v3/{}/messages", domain);
    let from= format!("{}@{}", config.mail.user, domain);
    let payload = MailgunPayload {
        from: from.to_string(),
        to: "kashish@rapidinnovation.dev".to_string(),
        subject: subject.to_string(),
        text: body.to_string(),
    };

    let res = client
        .post(&url)
        .basic_auth("api", Some(api_key)) 
        .form(&payload)
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => Ok(()),
        Ok(response) => {
            Err(CustomError::UnexpectedError(anyhow::anyhow!("Failed to send mail")))
        }
        Err(err) => Err(CustomError::UnexpectedError(anyhow::anyhow!("Failed to send mail"))),
    }
}
