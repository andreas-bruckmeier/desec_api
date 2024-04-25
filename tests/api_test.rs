use desec_api::Client;
use tokio::time::{sleep, Duration};

fn read_apikey() -> Option<String> {
    std::env::var("DESEC_API_TOKEN").ok()
}

fn read_domain() -> Option<String> {
    std::env::var("DESEC_DOMAIN").ok()
}

fn read_subname() -> Option<String> {
    std::env::var("DESEC_SUBNAME").ok()
}

#[tokio::test]
async fn test_account_info() {
    if let Some(key) = read_apikey() {
        let client = Client::new(key.clone()).unwrap();
        let account_info = client.account().get_account_info().await;
        println!("{:#?}", account_info);
        assert!(account_info.is_ok());
        assert!(account_info.unwrap().email.contains("@"));
    }
}

#[tokio::test]
async fn test_missing_resssources() {
    if let (Some(key), Some(domain)) = (read_apikey(), read_domain()) {
        let client = Client::new(key.clone()).unwrap();

        // Check missing rrset
        let rrset = client
            .rrset()
            .get_rrset(&domain, "non-existing-subname", "A")
            .await;
        match rrset {
            Err(desec_api::Error::NotFound) => (),
            _ => panic!("Should yield desec_api::Error::NotFound"),
        }

        sleep(Duration::from_millis(1000)).await;

        // Check missing rrset
        let rrset = client.domain().get_domain("non-existing-domain").await;
        match rrset {
            Err(desec_api::Error::NotFound) => (),
            _ => panic!("Should yield desec_api::Error::NotFound"),
        }
    }
}

#[tokio::test]
async fn test_rrset() {
    if let (Some(key), Some(domain), Some(subname)) = (read_apikey(), read_domain(), read_subname())
    {
        let client = Client::new(key.clone()).unwrap();
        let rrset_type = String::from("A");
        let records = [String::from("8.8.8.8")];

        let rrset = client
            .rrset()
            .create_rrset(
                domain.clone(),
                subname.clone(),
                rrset_type.clone(),
                &records,
                3600,
            )
            .await;

        assert!(rrset.is_ok());
        assert_eq!(rrset.as_ref().unwrap().domain.clone(), domain);
        assert_eq!(rrset.unwrap().records, records);

        sleep(Duration::from_millis(1000)).await;

        let rrset = client
            .rrset()
            .get_rrset(&domain, &subname, &rrset_type)
            .await;

        assert!(rrset.is_ok());
        let mut rrset = rrset.unwrap();

        assert_eq!(rrset.domain.clone(), domain);
        assert_eq!(rrset.records.clone(), records);

        rrset.ttl = 3650;

        std::thread::sleep(Duration::from_millis(1000));

        let rrset = client.rrset().patch_rrset_from(&rrset).await;

        assert!(rrset.is_ok());
        let rrset = rrset.unwrap().unwrap();

        assert_eq!(rrset.domain.clone(), domain);
        assert_eq!(rrset.ttl.clone(), 3650);

        std::thread::sleep(Duration::from_millis(1000));

        match client
            .rrset()
            .delete_rrset(&domain, &subname, &rrset_type)
            .await
        {
            Ok(_) => {}
            Err(err) => {
                println!("{:#?}", err);
            }
        }
    }
}
