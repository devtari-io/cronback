pub mod webhook_proto {
    tonic::include_proto!("webhook_proto");
}

pub mod scheduler_proto {
    tonic::include_proto!("scheduler_proto");
}

pub mod dispatcher_proto {
    tonic::include_proto!("dispatcher_proto");
}

pub mod trigger_proto {
    tonic::include_proto!("trigger_proto");
}

pub mod invocation_proto {
    tonic::include_proto!("invocation_proto");
}

pub mod attempt_proto {
    tonic::include_proto!("attempt_proto");
}

pub const FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("file_descriptor");
