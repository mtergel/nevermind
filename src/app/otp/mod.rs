pub mod email_otp;

/// Handle the storage logic, on own
pub trait OtpManager {
    fn generate_otp(&self) -> String;
}
