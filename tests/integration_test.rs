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
            let token =
                var("DESEC_TOKEN").expect("Envvar DESEC_TOKEN should be set with valid token");
            let mut client = Client::new(token).expect("Client should be buildable");
            client.set_max_wait_retry(60);
            client.set_max_retries(3);
            let domain = var("DESEC_DOMAIN").unwrap();
            TestConfiguration { client, domain }
        })
        .await
}

#[tokio::test]
async fn login_logout() {
    let email = var("DESEC_EMAIL").unwrap();
    let password = var("DESEC_PASSWORD").unwrap();
    let logged_in_client = Client::new_from_credentials(&email, &password)
        .await
        .expect("Login should not fail");
    logged_in_client
        .logout()
        .await
        .expect("Logout should not fail");
}

#[allow(clippy::needless_return)] // tokio_shared_rt somehow messes around
#[tokio_shared_rt::test(shared)]
async fn account_info() {
    let config = get_config().await;
    let account_info = config.client.account().get_account_info().await;
    let account_info = account_info.expect("account_info should be ok");
    let expected: AccountInformation = serde_json::from_str(&var("DESEC_ACCOUNT_INFO").expect(""))
        .expect("expected account_info should be deserializable");
    assert_eq!(account_info, expected);
}

#[allow(clippy::needless_return)] // tokio_shared_rt somehow messes around
#[tokio_shared_rt::test(shared)]
async fn zonefile() {
    let config = get_config().await;
    let zonefile = config
        .client
        .domain()
        .get_zonefile(&config.domain)
        .await
        .expect("Zonefile should be exportable");
    assert!(
        zonefile.contains("exported from desec.io"),
        "Zonefile does not contain expected string"
    );
}

#[allow(clippy::needless_return)]
#[tokio_shared_rt::test(shared)]
async fn captcha() {
    let res = desec_api::account::get_captcha().await;
    assert!(res.is_ok());
    let captcha = res.unwrap();
    assert_eq!(captcha.kind, desec_api::account::CaptchaKind::Image);
}

#[allow(clippy::needless_return)]
#[tokio_shared_rt::test(shared)]
async fn missing_resssources() {
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
    };
}

#[allow(clippy::needless_return)]
#[tokio_shared_rt::test(shared)]
async fn rrset() {
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

#[allow(clippy::needless_return)]
#[tokio_shared_rt::test(shared)]
async fn rrset_at_apex() {
    let config = get_config().await;
    let records = vec!["\"This is a test\"".to_string()];
    let rrset = config
        .client
        .rrset()
        .create_rrset(&config.domain, None, "TXT", 3600, &records)
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
        .get_rrset(&config.domain, None, "TXT")
        .await;

    assert!(rrset.is_ok());
    let rrset = rrset.unwrap();

    assert_eq!(rrset.domain.clone(), config.domain);
    assert_eq!(rrset.records.clone(), records);

    // Respect rate limit
    sleep(Duration::from_millis(1000)).await;

    let res = config
        .client
        .rrset()
        .delete_rrset(&config.domain, None, "TXT")
        .await;
    res.expect("should be ok");
}

#[allow(clippy::needless_return)]
#[tokio_shared_rt::test(shared)]
async fn retrieve_token() {
    let config = get_config().await;
    let token = config
        .client
        .token()
        .get(
            var("DESEC_TOKEN_ID")
                .expect("Envvar DESEC_TOKEN_ID should be set with valid token")
                .as_str(),
        )
        .await;
    token.expect("token should be ok");
}

#[allow(clippy::needless_return)]
#[tokio_shared_rt::test(shared)]
async fn patch_token() {
    let config = get_config().await;
    let token_new_name = format!("token-{}", Uuid::new_v4());
    config
        .client
        .token()
        .patch(
            var("DESEC_TOKEN_ID")
                .expect("Envvar DESEC_TOKEN_ID should be set with valid token")
                .as_str(),
            Some(token_new_name.clone()),
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Token should be patchable");
}

#[allow(clippy::needless_return)]
#[tokio_shared_rt::test(shared)]
async fn create_and_delete_token() {
    let config = get_config().await;
    let token = config
        .client
        .token()
        .create(
            Some(format!("integrationtest-{}", Uuid::new_v4())),
            None,
            None,
            None,
            None,
        )
        .await;
    let token = token.expect("token should be ok");

    // Respect rate limit
    sleep(Duration::from_millis(1000)).await;

    // Delete token
    let token = config.client.token().delete(token.id.as_str()).await;
    token.expect("token delete should be ok");
}

#[allow(clippy::needless_return)]
#[tokio_shared_rt::test(shared)]
async fn token_policy() {
    let config = get_config().await;

    // Create token
    let token = config
        .client
        .token()
        .create(
            Some(format!("integrationtest-{}", Uuid::new_v4())),
            None,
            None,
            None,
            None,
        )
        .await;
    let token = token.expect("token should be ok");

    sleep(Duration::from_millis(1000)).await;

    // Create policy
    let policy = config
        .client
        .token()
        .create_policy(&token.id, None, None, None, None)
        .await;
    let policy = policy.expect("token policy should be ok");

    sleep(Duration::from_millis(1000)).await;

    // Get
    let policy_get = config
        .client
        .token()
        .get_policy(&token.id, &policy.id)
        .await;
    let policy_get = policy_get.expect("Fetch of token policy should be ok");
    assert_eq!(policy_get.id, policy.id);
    assert_eq!(policy_get.domain, policy.domain);
    assert_eq!(policy_get.subname, policy.subname);
    assert_eq!(policy_get.r#type, policy.r#type);
    assert_eq!(policy_get.perm_write, policy.perm_write);

    sleep(Duration::from_millis(1000)).await;

    // Delete policy
    let res = config
        .client
        .token()
        .delete_policy(&token.id, &policy.id)
        .await;
    res.expect("Deletion of token policy should be ok");

    sleep(Duration::from_millis(1000)).await;

    // Delete token
    let res = config.client.token().delete(&token.id).await;
    res.expect("Deletion of token should be ok");
}
