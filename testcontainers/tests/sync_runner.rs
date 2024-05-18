#![cfg(feature = "blocking")]

use testcontainers::{
    core::{CmdWaitFor, ExecCommand, Host, WaitFor},
    runners::SyncRunner,
    *,
};

fn get_server_container(msg: Option<WaitFor>) -> GenericImage {
    let msg = msg.unwrap_or(WaitFor::message_on_stdout("server is ready"));
    GenericImage::new("simple_web_server", "latest").with_wait_for(msg)
}

#[derive(Debug, Default)]
pub struct HelloWorld;

impl Image for HelloWorld {
    type Args = ();

    fn name(&self) -> String {
        "hello-world".to_owned()
    }

    fn tag(&self) -> String {
        "latest".to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Hello from Docker!")]
    }
}

#[test]
fn sync_can_run_hello_world() {
    let _ = pretty_env_logger::try_init();
    let _container = HelloWorld.start();
}

#[test]
fn generic_image_with_custom_entrypoint() {
    let generic = get_server_container(None);

    let node = generic.start();
    let port = node.get_host_port_ipv4(80);
    assert_eq!(
        "foo",
        reqwest::blocking::get(format!("http://{}:{port}", node.get_host()))
            .unwrap()
            .text()
            .unwrap()
    );

    let generic = get_server_container(None).with_entrypoint("./bar");

    let node = generic.start();
    let port = node.get_host_port_ipv4(80);
    assert_eq!(
        "bar",
        reqwest::blocking::get(format!("http://{}:{port}", node.get_host()))
            .unwrap()
            .text()
            .unwrap()
    );
}

#[test]
fn generic_image_exposed_ports() {
    let _ = pretty_env_logger::try_init();

    let target_port = 8080;

    // This server does not EXPOSE ports in its image.
    let generic_server = GenericImage::new("no_expose_port", "latest")
        .with_wait_for(WaitFor::message_on_stdout("listening on 0.0.0.0:8080"))
        // Explicitly expose the port, which otherwise would not be available.
        .with_exposed_port(target_port);

    let node = generic_server.start();
    let port = node.get_host_port_ipv4(target_port);
    assert!(reqwest::blocking::get(format!("http://127.0.0.1:{port}"))
        .unwrap()
        .status()
        .is_success());
}

#[test]
fn generic_image_running_with_extra_hosts_added() {
    let server_1 = get_server_container(None);
    let node = server_1.start();
    let port = node.get_host_port_ipv4(80);

    let msg = WaitFor::message_on_stdout("foo");
    let server_2 = GenericImage::new("curlimages/curl", "latest")
        .with_wait_for(msg)
        .with_entrypoint("curl");

    // Override hosts for server_2 adding
    // custom-host as an alias for localhost
    let server_2 = RunnableImage::from((server_2, vec![format!("http://custom-host:{port}")]))
        .with_host("custom-host", Host::HostGateway);

    server_2.start();
}

#[test]
#[should_panic]
fn generic_image_port_not_exposed() {
    let _ = pretty_env_logger::try_init();

    let target_port = 8080;

    // This image binds to 0.0.0.0:8080, does not EXPOSE ports in its dockerfile.
    let generic_server = GenericImage::new("no_expose_port", "latest")
        .with_wait_for(WaitFor::message_on_stdout("listening on 0.0.0.0:8080"));
    let node = generic_server.start();

    // Without exposing the port with `with_exposed_port()`, we cannot get a mapping to it.
    node.get_host_port_ipv4(target_port);
}

#[test]
fn start_multiple_containers() {
    let _ = pretty_env_logger::try_init();

    let image = GenericImage::new("hello-world", "latest").with_wait_for(WaitFor::seconds(2));

    let _container_1 = image.clone().start();
    let _container_2 = image.clone().start();
    let _container_3 = image.start();
}

#[test]
fn sync_run_exec() {
    let _ = pretty_env_logger::try_init();

    let image = GenericImage::new("simple_web_server", "latest")
        .with_wait_for(WaitFor::message_on_stdout("server is ready"))
        .with_wait_for(WaitFor::seconds(1));
    let container = image.start();

    // exit code, it waits for result
    let res = container
        .exec(
            ExecCommand::new(vec!["sleep".to_string(), "3".to_string()])
                .with_cmd_ready_condition(CmdWaitFor::exit_code(0)),
        )
        .unwrap();
    assert_eq!(res.exit_code().unwrap(), Some(0));

    // stdout
    let mut res = container
        .exec(
            ExecCommand::new(vec!["ls".to_string()])
                .with_cmd_ready_condition(CmdWaitFor::message_on_stdout("foo")),
        )
        .unwrap();
    assert_eq!(res.exit_code().unwrap(), Some(0));
    let stdout = String::from_utf8(res.stdout().unwrap()).unwrap();
    assert!(stdout.contains("foo"), "stdout must contain 'foo'");

    // stdout and stderr readers
    let mut res = container
        .exec(ExecCommand::new([
            "/bin/bash",
            "-c",
            "echo 'stdout 1' >&1 && echo 'stderr 1' >&2 \
            && echo 'stderr 2' >&2 && echo 'stdout 2' >&1",
        ]))
        .unwrap();

    let mut stdout = String::new();
    res.stdout_reader().read_to_string(&mut stdout).unwrap();
    assert_eq!(stdout, "stdout 1\nstdout 2\n");

    let mut stderr = String::new();
    res.stderr_reader().read_to_string(&mut stderr).unwrap();
    assert_eq!(stderr, "stderr 1\nstderr 2\n");
}
