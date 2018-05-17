#[macro_use] extern crate lazy_static;

extern crate env_logger;
extern crate hyper_client_pool;
extern crate ipnet;
extern crate regex;

use std::net::IpAddr;
use std::process::Command;
use std::sync::{Arc, mpsc};
use std::sync::atomic::{Ordering, AtomicUsize};
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

use hyper_client_pool::*;
use hyper::{Request, Method};
use ipnet::{Contains, IpNet};
use regex::Regex;

lazy_static! {
    /// For tests that depend on global state (ahem - keep_alive_works_as_expected())
    /// we have this test_lock which they can grab with a `write()` in order to ensure
    /// no other tests in this file are running
    static ref TEST_LOCK: RwLock<()> = RwLock::new(());
}

#[derive(Debug)]
struct MspcDeliverable(mpsc::Sender<DeliveryResult>);

impl Deliverable for MspcDeliverable {
    fn complete(self, result: DeliveryResult) {
        let _ = self.0.send(result);
    }
}

fn default_config() -> Config {
    Config {
        keep_alive_timeout: Duration::from_secs(3),
        transaction_timeout: Duration::from_secs(10),
        max_transactions_per_worker: 1_000,
        workers: 2,
    }
}

fn onesignal_transaction<D: Deliverable>(deliverable: D) -> Transaction<D> {
    Transaction::new(deliverable, Request::new(Method::Get, "https://onesignal.com/".parse().unwrap()))
}

fn assert_successful_result(result: DeliveryResult) {
    match result {
        DeliveryResult::Response { response, .. } => {
            assert!(response.status().is_success(), format!("Expected successful response: {:?}", response.status()));
        },
        res => panic!("Expected DeliveryResult::Response, unexpected delivery result: {:?}", res),
    }
}

#[test]
fn a_ton_of_notifications() {
    let _read = TEST_LOCK.read().unwrap_or_else(|e| e.into_inner());

    let _ = env_logger::try_init();

    let mut config = default_config();
    config.workers = 2;

    let mut pool = Pool::new(config).unwrap();
    let (tx, rx) = mpsc::channel();

    for _ in 0..2000 {
        pool.request(onesignal_transaction(MspcDeliverable(tx.clone()))).expect("request ok");
    }

    for _ in 0..2000 {
        assert_successful_result(rx.recv().unwrap());
    }
}

#[test]
fn lots_of_get_single_worker() {
    let _read = TEST_LOCK.read().unwrap_or_else(|e| e.into_inner());

    let _ = env_logger::try_init();

    let mut config = default_config();
    config.workers = 1;

    let mut pool = Pool::new(config).unwrap();
    let (tx, rx) = mpsc::channel();

    for _ in 0..5 {
        pool.request(onesignal_transaction(MspcDeliverable(tx.clone()))).expect("request ok");
    }

    for _ in 0..5 {
        assert_successful_result(rx.recv().unwrap());
    }
}

#[derive(Debug, Clone)]
struct SuccessfulCompletionCounter {
    count: Arc<AtomicUsize>,
}

impl SuccessfulCompletionCounter {
    fn new() -> SuccessfulCompletionCounter {
        SuccessfulCompletionCounter { count: Arc::new(AtomicUsize::new(0)) }
    }

    fn count(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }
}

impl Deliverable for SuccessfulCompletionCounter {
    fn complete(self, result: DeliveryResult) {
        assert_successful_result(result);
        self.count.fetch_add(1, Ordering::AcqRel);
    }
}

#[test]
fn graceful_shutdown() {
    let _read = TEST_LOCK.read().unwrap_or_else(|e| e.into_inner());

    let _ = env_logger::try_init();

    let txn = 20;
    let counter = SuccessfulCompletionCounter::new();

    let mut config = default_config();
    config.workers = 2;

    let mut pool = Pool::new(config).unwrap();
    for _ in 0..txn {
        pool.request(onesignal_transaction(counter.clone())).expect("request ok");
    }

    pool.shutdown();
    assert_eq!(counter.count(), txn);
}

#[test]
fn full_error() {
    let _read = TEST_LOCK.read().unwrap_or_else(|e| e.into_inner());

    let _ = env_logger::try_init();

    let mut config = default_config();
    config.workers = 3;
    config.max_transactions_per_worker = 1;

    let mut pool = Pool::new(config).unwrap();
    let (tx, rx) = mpsc::channel();

    // Start requests
    for _ in 0..3 {
        pool.request(onesignal_transaction(MspcDeliverable(tx.clone()))).expect("request ok");
    }

    match pool.request(onesignal_transaction(MspcDeliverable(tx.clone()))) {
        Err(err) => assert_eq!(err.kind, ErrorKind::PoolFull),
        _ => panic!("Expected Error, got success request!"),
    }

    for _ in 0..3 {
        assert_successful_result(rx.recv().unwrap());
    }
}

static CLOUDFLARE_NETS: &[&str] = &[
    // IPv4
    "103.21.244.0/22",
    "103.22.200.0/22",
    "103.31.4.0/22",
    "104.16.0.0/12",
    "108.162.192.0/18",
    "131.0.72.0/22",
    "141.101.64.0/18",
    "162.158.0.0/15",
    "172.64.0.0/13",
    "173.245.48.0/20",
    "188.114.96.0/20",
    "190.93.240.0/20",
    "197.234.240.0/22",
    "198.41.128.0/17",

    // IPv6
    "2400:cb00::/32",
    "2405:8100::/32",
    "2405:b500::/32",
    "2606:4700::/32",
    "2803:f800::/32",
    "2c0f:f248::/32",
    "2a06:98c0::/29",
];

lazy_static! {
    static ref CLOUDFLARE_PARSED_NETS: Vec<IpNet> = {
        CLOUDFLARE_NETS.iter()
            .map(|net| net.parse::<IpNet>())
            .collect::<Result<Vec<IpNet>, _>>().unwrap()
    };

    static ref LSOF_PARSE_IP_REGEX: Regex = {
        Regex::new(r"->\[?([^\]]*)\]?:https").unwrap()
    };
}

fn matches_cloudflare_ip(input: &str) -> bool {
    if let Some(captures) = LSOF_PARSE_IP_REGEX.captures(input) {
        match captures[1].parse::<IpAddr>() {
            Ok(addr) => CLOUDFLARE_PARSED_NETS.iter().any(|net| net.contains(&addr)),
            Err(_err) => false,
        }
    } else {
        false
    }
}

#[test]
fn matches_cloudflare_ip_works_as_expected() {
    // Test-case from staging
    let input1 = "hyper_cli 29606 deploy    9u  IPv6 74567336      0t0  TCP onepush-test-darren:46286->[2400:cb00:2048:1::6810:cea5]:https (ESTABLISHED)";
    assert_eq!(matches_cloudflare_ip(input1), true);
    // test-case from personal mac
    let input2 = "lib-f13ca 83600 darrentsung   12u  IPv4 0x2aad2644e2239ff9      0t0  TCP 192.168.2.240:54285->104.16.207.165:https (ESTABLISHED)";
    assert_eq!(matches_cloudflare_ip(input2), true);
}

fn onesignal_connection_count() -> (usize, String) {
    let output = Command::new("lsof")
        .args(&["-i"])
        .output()
        .expect("command works");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines : Vec<_> = stdout.split("\n")
        .filter(|line| matches_cloudflare_ip(line))
        .collect();

    let stdout = lines.join("\n");

    (lines.len(), stdout)
}

macro_rules! assert_onesignal_connection_open_count_eq {
    ($expected_open_count:expr) => {
        let (open_count, stdout) = onesignal_connection_count();
        assert_eq!($expected_open_count, open_count, "Output:\n{}", stdout);
    };
}

#[test]
fn keep_alive_works_as_expected() {
    let _write = TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());

    // block until no connections are open - this is unfortunate..
    // but at least we have tests covering the keep-alive :)
    while onesignal_connection_count().0 > 0 {}

    let _ = env_logger::try_init();

    let mut config = default_config();
    config.keep_alive_timeout = Duration::from_secs(3);

    let mut pool = Pool::new(config).unwrap();
    let (tx, rx) = mpsc::channel();

    // Start first request
    pool.request(onesignal_transaction(MspcDeliverable(tx.clone()))).expect("request ok");

    // wait for request to finish
    assert_successful_result(rx.recv().unwrap());
    thread::sleep(Duration::from_secs(1));
    assert_onesignal_connection_open_count_eq!(1);
    thread::sleep(Duration::from_secs(1));
    assert_onesignal_connection_open_count_eq!(1);

    thread::sleep(Duration::from_secs(7));
    // keep-alive should kill connection by now
    assert_onesignal_connection_open_count_eq!(0);
}

#[test]
fn connection_reuse_works_as_expected() {
    let _write = TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());

    // block until no connections are open - this is unfortunate..
    // but at least we have tests covering the keep-alive :)
    while onesignal_connection_count().0 > 0 {}

    let _ = env_logger::try_init();

    let mut config = default_config();
    // note that workers must be one otherwise the second transaction will be
    // routed to another worker and a new connection will be established
    config.workers = 1;
    config.keep_alive_timeout = Duration::from_secs(10);

    let mut pool = Pool::new(config).unwrap();
    let (tx, rx) = mpsc::channel();

    // Start first request
    pool.request(onesignal_transaction(MspcDeliverable(tx.clone()))).expect("request ok");
    // wait for request to finish
    assert_successful_result(rx.recv().unwrap());

    assert_onesignal_connection_open_count_eq!(1);
    thread::sleep(Duration::from_secs(3));
    assert_onesignal_connection_open_count_eq!(1);

    // Start second request
    pool.request(onesignal_transaction(MspcDeliverable(tx.clone()))).expect("request ok");
    // wait for request to finish
    assert_successful_result(rx.recv().unwrap());

    // there should only be one connection open
    assert_onesignal_connection_open_count_eq!(1);
}

#[test]
fn timeout_works_as_expected() {
    let _read = TEST_LOCK.read().unwrap_or_else(|e| e.into_inner());

    let _ = env_logger::try_init();

    let mut config = default_config();
    config.transaction_timeout = Duration::from_secs(2);

    let mut pool = Pool::new(config).unwrap();
    let (tx, rx) = mpsc::channel();

    // Start first request
    pool.request(
        // This endpoint will not return for a while, therefore should timeout
        Transaction::new(MspcDeliverable(tx.clone()), Request::new(Method::Get, "https://httpstat.us/200?sleep=5000".parse().unwrap()))
    ).expect("request ok");

    match rx.recv().unwrap() {
        DeliveryResult::Timeout { .. } => (), // ok
        res => panic!("Expected timeout!, got: {:?}", res),
    }
}
