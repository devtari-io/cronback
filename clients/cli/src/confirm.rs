static ACCEPTABLE_YES: &[&str] = &["y", "yes", "Y", "YES"];

pub fn confirm_fn<T>(always_yes: bool, msg: T) -> bool
where
    T: AsRef<str>,
{
    if always_yes {
        return true;
    }
    let answer =
        rprompt::prompt_reply(format!("{} [y/N] ", msg.as_ref())).unwrap();
    if ACCEPTABLE_YES.contains(&answer.as_str()) {
        return true;
    }
    false
}

#[allow(unused_macros)]
#[rustfmt::skip]
macro_rules! confirm {
    ($self:ident, $($arg:tt)*) => {{
        let res = ::std::format!($($arg)*);
        $crate::confirm::confirm_fn($self.yes, res)
    }}
}

#[rustfmt::skip]
macro_rules! confirm_or_abort {
    ($opts:ident, $($arg:tt)*) => {{
        let res = ::std::format!($($arg)*);
        if !$crate::confirm::confirm_fn($opts.yes, res) {
            return Err(::anyhow::anyhow!("Aborted!"));
        }
    }}
}

#[allow(unused)]
pub(crate) use {confirm, confirm_or_abort};
