use desec_api::account::AccountInformation;
use desec_api::Client;
use std::env::var;
use tokio::sync::OnceCell;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

struct TestConfiguration {
    client: Client,
    domain: String,
}

static CONFIG: OnceCell<TestConfiguration> = OnceCell::const_new();

// Creates a configuration that can be used by all tests.
// As tests are run independent from each other but we want
// to only create one configuration for all tests, so we use tokio::sync::OnceCell
// to create some kind of a singleton function.
async fn get_config() -> &'static TestConfiguration {
    CONFIG
        .get_or_init(|| async {
            let email = var("DESEC_EMAIL").unwrap();
            let password = var("DESEC_PASSWORD").unwrap();
            let domain = var("DESEC_DOMAIN").unwrap();
            let client = Client::new_from_credentials(&email, &password)
                .await
                .expect("Authentication should succeed");
            TestConfiguration {
                client,
                domain,
            }
        })
        .await
}

#[tokio_shared_rt::test(shared)]
async fn test_account_info() {
    let config = get_config().await;
    let account_info = config.client.account().get_account_info().await;
    let account_info = account_info.expect("account_info should be ok");
    let expected: AccountInformation = serde_json::from_str(&var("DESEC_ACCOUNT_INFO").expect(""))
        .expect("expected account_info should be deserializable");
    assert_eq!(account_info, expected);
}

#[tokio_shared_rt::test(shared)]
async fn test_captcha() {
    let res = desec_api::account::get_captcha().await;
    assert!(res.is_ok());
    let captcha = res.unwrap();
    assert_eq!(captcha.kind, desec_api::account::CaptchaKind::Image);
}

#[tokio_shared_rt::test(shared)]
async fn test_missing_resssources() {
    let config = get_config().await;

    // Check missing rrset
    let rrset = config
        .client
        .rrset()
        .get_rrset(&config.domain, Some("non-existing-subname"), "A")
        .await;
    match rrset {
        Err(desec_api::Error::NotFound) => (),
        _ => panic!("Should yield desec_api::Error::NotFound"),
    }

    // Check missing rrset
    let rrset = config
        .client
        .domain()
        .get_domain("non-existing-domain")
        .await;
    match rrset {
        Err(desec_api::Error::NotFound) => (),
        _ => panic!("Should yield desec_api::Error::NotFound"),
    }
}

#[tokio_shared_rt::test(shared)]
async fn test_rrset() {
    let config = get_config().await;
    // Random subname
    let subname = format!("test-{}", Uuid::new_v4());
    let rrset_type = String::from("A");
    let records = vec![String::from("8.8.8.8")];

    let rrset = config
        .client
        .rrset()
        .create_rrset(&config.domain, Some(&subname), &rrset_type, 3600, &records)
        .await;

    assert!(rrset.is_ok());
    let rrset = rrset.unwrap();
    assert_eq!(rrset.domain.clone(), config.domain);
    assert_eq!(rrset.records, records);

    // Respect rate limit
    sleep(Duration::from_millis(1000)).await;

    let rrset = config
        .client
        .rrset()
        .get_rrset(&config.domain, Some(&subname), &rrset_type)
        .await;

    assert!(rrset.is_ok());
    let mut rrset = rrset.unwrap();

    assert_eq!(rrset.domain.clone(), config.domain);
    assert_eq!(rrset.records.clone(), records);

    rrset.ttl = 3650;

    // Respect rate limit
    sleep(Duration::from_millis(1000)).await;

    let rrset = config.client.rrset().patch_rrset_from(&rrset).await;

    assert!(rrset.is_ok());
    let rrset = rrset.unwrap().unwrap();

    assert_eq!(rrset.domain.clone(), config.domain);
    assert_eq!(rrset.ttl.clone(), 3650);

    // Respect rate limit
    sleep(Duration::from_millis(1000)).await;

    let res = config
        .client
        .rrset()
        .delete_rrset(&config.domain, Some(&subname), &rrset_type)
        .await;
    res.expect("should be ok");
}

#[tokio_shared_rt::test(shared)]
async fn test_retrieve_token() {
    let config = get_config().await;
    let token = config.client.token().get(
        "fd486071-ec30-42c3-bb95-63e4d07f1b19"
    ).await;
    let _ = token.expect("token should be ok");
}

#[tokio_shared_rt::test(shared)]
async fn test_create_and_delete_token() {
    let config = get_config().await;
    let token = config.client.token().create_token(
        Some(format!("integrationtest-{}", Uuid::new_v4())),
        None,
        None,
        None,
        None
    ).await;
    let token = token.expect("token should be ok");

    // Respect rate limit
    sleep(Duration::from_millis(1000)).await;

    // Delete token
    let token = config.client.token().delete_token(
        token.id.as_str()
    ).await;
    let _ = token.expect("token delete should be ok");
}
