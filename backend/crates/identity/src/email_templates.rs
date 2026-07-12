//! Transactional identity email content.

pub(crate) struct EmailContent {
    pub subject: &'static str,
    pub text: String,
    pub html: String,
}

pub(crate) fn login_code(code: &str) -> EmailContent {
    code_email("YourTJ 登录验证码", "使用以下验证码完成校园邮箱登录或注册。", code)
}

pub(crate) fn password_reset_code(code: &str) -> EmailContent {
    code_email("YourTJ 密码重置", "使用以下验证码重置您的 YourTJ 密码。", code)
}

pub(crate) fn recent_auth_code(code: &str) -> EmailContent {
    code_email("YourTJ 安全验证", "使用以下验证码确认当前设备上的高风险操作。", code)
}

pub(crate) fn appeal_code(code: &str) -> EmailContent {
    code_email(
        "YourTJ 申诉验证",
        "使用以下验证码进入申诉中心。该验证码不会登录其他 YourTJ 功能。",
        code,
    )
}

pub(crate) fn account_recovery_code(code: &str) -> EmailContent {
    code_email(
        "YourTJ 账号恢复",
        "使用以下验证码恢复已停用或处于删除恢复期的账号。该验证码不会直接登录 YourTJ。",
        code,
    )
}

pub(crate) fn community_invitation() -> EmailContent {
    EmailContent {
        subject: "YourTJ 社区邀请",
        text: "管理员已为您预留 YourTJ 账号。请使用校园邮箱验证码完成邮箱所有权验证和首次登录。"
            .into(),
        html: email_shell(
            "社区邀请",
            "管理员已为您预留 YourTJ 账号。请使用校园邮箱验证码完成邮箱所有权验证和首次登录。",
            None,
        ),
    }
}

fn code_email(subject: &'static str, introduction: &str, code: &str) -> EmailContent {
    EmailContent {
        subject,
        text: format!(
            "{introduction}\n\n验证码：{code}\n\n验证码 10 分钟内有效。如非本人操作，请忽略此邮件。"
        ),
        html: email_shell(subject.trim_start_matches("YourTJ "), introduction, Some(code)),
    }
}

fn email_shell(title: &str, introduction: &str, code: Option<&str>) -> String {
    let code_block = code.map_or_else(String::new, |value| {
        format!(
            "<div style=\"margin:24px 0;padding:18px 20px;border-radius:12px;background:#f4f8f5;\
             color:#183b2a;font-size:30px;font-weight:700;letter-spacing:8px;text-align:center\">\
             {value}</div>"
        )
    });
    format!(
        "<!doctype html><html><body style=\"margin:0;background:#f5f5f2;color:#20231f;\
         font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif\">\
         <div style=\"max-width:560px;margin:32px auto;padding:32px;border-radius:16px;background:#fff\">\
         <p style=\"margin:0 0 8px;color:#2f7652;font-size:14px;font-weight:700\">YOURTJ COMMUNITY</p>\
         <h1 style=\"margin:0 0 16px;font-size:24px\">{title}</h1>\
         <p style=\"margin:0;line-height:1.7\">{introduction}</p>{code_block}\
         <p style=\"margin:24px 0 0;color:#6b716c;font-size:13px;line-height:1.6\">\
         验证码 10 分钟内有效。如非本人操作，请忽略此邮件。</p></div></body></html>"
    )
}
