error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Discord(::discord::Error);
        Redis(::redis::RedisError);
    }
}
