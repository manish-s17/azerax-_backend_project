use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use crate::utils::errors::AppError;

#[derive(Clone)]
pub struct EmailService {
    pub smtp_host:  String,
    pub smtp_port:  u16,
    pub smtp_user:  String,
    pub smtp_pass:  String,
    pub from_email: String,
    pub app_url:    String,
}

impl EmailService {
    pub fn new(
        smtp_host: String,
        smtp_port: u16,
        smtp_user: String,
        smtp_pass: String,
        from_email: String,
        app_url: String,
    ) -> Self {
        Self { smtp_host, smtp_port, smtp_user, smtp_pass, from_email, app_url }
    }

    fn mailer(&self) -> Result<AsyncSmtpTransport<Tokio1Executor>, AppError> {
        let creds = Credentials::new(self.smtp_user.clone(), self.smtp_pass.clone());
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.smtp_host)
            .map_err(|e| AppError::Email(e.to_string()))?
            .port(self.smtp_port)
            .credentials(creds)
            .build()
            .pipe_ok()
    }

    pub async fn send_verification_email(&self, to: &str, username: &str, token: &str) -> Result<(), AppError> {
        let url = format!("{}/api/auth/verify-email?token={}", self.app_url, token);
        let html = format!(
            r#"<div style="font-family:sans-serif;max-width:500px;margin:auto;padding:32px;background:#0b0c10;color:#fff;border-radius:12px;">
              <h1 style="color:#8b5cf6;letter-spacing:4px;">AEZARX</h1>
              <h2>Verify your email</h2>
              <p>Hi <strong>{username}</strong>, thanks for joining AEZARX!</p>
              <p>Click the button below to verify your email address:</p>
              <a href="{url}" style="display:inline-block;margin:16px 0;padding:12px 28px;background:#8b5cf6;color:#fff;text-decoration:none;border-radius:8px;font-weight:bold;">
                Verify Email
              </a>
              <p style="color:#888;font-size:12px;">This link expires in 24 hours. If you didn't register, ignore this email.</p>
              <p style="color:#888;font-size:11px;">Or copy: {url}</p>
            </div>"#
        );
        self.send(to, "Verify your AEZARX account", &html).await
    }

    pub async fn send_password_reset_email(&self, to: &str, username: &str, token: &str) -> Result<(), AppError> {
        let url = format!("{}/reset-password?token={}", self.app_url, token);
        let html = format!(
            r#"<div style="font-family:sans-serif;max-width:500px;margin:auto;padding:32px;background:#0b0c10;color:#fff;border-radius:12px;">
              <h1 style="color:#8b5cf6;letter-spacing:4px;">AEZARX</h1>
              <h2>Reset your password</h2>
              <p>Hi <strong>{username}</strong>, we received a request to reset your password.</p>
              <a href="{url}" style="display:inline-block;margin:16px 0;padding:12px 28px;background:#ef4444;color:#fff;text-decoration:none;border-radius:8px;font-weight:bold;">
                Reset Password
              </a>
              <p style="color:#888;font-size:12px;">This link expires in 1 hour. If you didn't request this, ignore this email.</p>
              <p style="color:#888;font-size:11px;">Or copy: {url}</p>
            </div>"#
        );
        self.send(to, "Reset your AEZARX password", &html).await
    }

    pub async fn send_magic_link_email(&self, to: &str, username: &str, token: &str) -> Result<(), AppError> {
        let url = format!("{}/magic-login?token={}", self.app_url, token);
        let html = format!(
            r#"<div style="font-family:sans-serif;max-width:500px;margin:auto;padding:32px;background:#0b0c10;color:#fff;border-radius:12px;">
              <h1 style="color:#8b5cf6;letter-spacing:4px;">AEZARX</h1>
              <h2>Your magic sign-in link</h2>
              <p>Hi <strong>{username}</strong>! Click below to sign in instantly — no password needed.</p>
              <a href="{url}" style="display:inline-block;margin:16px 0;padding:12px 28px;background:#8b5cf6;color:#fff;text-decoration:none;border-radius:8px;font-weight:bold;">
                Sign In to AEZARX
              </a>
              <p style="color:#888;font-size:12px;">This link expires in 15 minutes and can only be used once.</p>
              <p style="color:#888;font-size:11px;">Or copy: {url}</p>
            </div>"#
        );
        self.send(to, "Your AEZARX magic sign-in link", &html).await
    }

    async fn send(&self, to: &str, subject: &str, html: &str) -> Result<(), AppError> {
        let email = Message::builder()
            .from(self.from_email.parse().map_err(|e| AppError::Email(format!("Bad from addr: {e}")))?)
            .to(to.parse().map_err(|e| AppError::Email(format!("Bad to addr: {e}")))?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html.to_string())
            .map_err(|e| AppError::Email(e.to_string()))?;

        let mailer = self.mailer()?;
        mailer.send(email).await.map_err(|e| AppError::Email(e.to_string()))?;
        Ok(())
    }
}

// Helper trait to make the builder pattern cleaner
trait PipeOk<T> {
    fn pipe_ok(self) -> Result<T, AppError>;
}
impl<T> PipeOk<T> for T {
    fn pipe_ok(self) -> Result<T, AppError> { Ok(self) }
}
