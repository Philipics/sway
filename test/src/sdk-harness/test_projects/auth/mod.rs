use fuels::{
    accounts::{
        predicate::Predicate,
        wallet::{Wallet, WalletUnlocked},
    },
    prelude::*,
    tx::UtxoId,
    types::{
        coin::{Coin, CoinStatus},
        message::{Message, MessageStatus},
        Bytes32, ContractId,
    },
};
use std::str::FromStr;

abigen!(
    Contract(
        name = "AuthContract",
        abi = "test_artifacts/auth_testing_contract/out/release/auth_testing_contract-abi.json"
    ),
    Contract(
        name = "AuthCallerContract",
        abi = "test_artifacts/auth_caller_contract/out/release/auth_caller_contract-abi.json"
    ),
    Predicate(
        name = "AuthPredicate",
        abi = "test_artifacts/auth_predicate/out/release/auth_predicate-abi.json"
    ),
);

#[tokio::test]
async fn is_external_from_sdk() {
    let (auth_instance, _, _, _, _) = get_contracts().await;
    let result = auth_instance
        .methods()
        .is_caller_external()
        .call()
        .await
        .unwrap();

    assert!(result.value);
}

#[tokio::test]
async fn msg_sender_from_sdk() {
    let (auth_instance, _, _, _, wallet) = get_contracts().await;
    let result = auth_instance
        .methods()
        .returns_msg_sender_address(wallet.address())
        .call()
        .await
        .unwrap();

    assert!(result.value);
}

#[tokio::test]
async fn msg_sender_from_contract() {
    let (auth_instance, auth_id, caller_instance, caller_id, _) = get_contracts().await;

    let result = caller_instance
        .methods()
        .call_auth_contract(auth_id, caller_id)
        .with_contracts(&[&auth_instance])
        .call()
        .await
        .unwrap();

    assert!(result.value);
}

async fn get_contracts() -> (
    AuthContract<WalletUnlocked>,
    ContractId,
    AuthCallerContract<WalletUnlocked>,
    ContractId,
    Wallet,
) {
    let wallet = launch_provider_and_get_wallet().await.unwrap();

    let id_1 = Contract::load_from(
        "test_artifacts/auth_testing_contract/out/release/auth_testing_contract.bin",
        LoadConfiguration::default(),
    )
    .unwrap()
    .deploy(&wallet, TxPolicies::default())
    .await
    .unwrap();

    let id_2 = Contract::load_from(
        "test_artifacts/auth_caller_contract/out/release/auth_caller_contract.bin",
        LoadConfiguration::default(),
    )
    .unwrap()
    .deploy(&wallet, TxPolicies::default())
    .await
    .unwrap();

    let instance_1 = AuthContract::new(id_1.clone(), wallet.clone());
    let instance_2 = AuthCallerContract::new(id_2.clone(), wallet.clone());

    (
        instance_1,
        id_1.into(),
        instance_2,
        id_2.into(),
        wallet.lock(),
    )
}

#[tokio::test]
async fn can_get_predicate_address() {
    // Setup Wallets
    let asset_id = AssetId::default();
    let wallets_config = WalletsConfig::new_multiple_assets(
        2,
        vec![AssetConfig {
            id: asset_id,
            num_coins: 1,
            coin_amount: 1_000,
        }],
    );
    let wallets = &launch_custom_provider_and_get_wallets(wallets_config, None, None)
        .await
        .unwrap();
    let first_wallet = &wallets[0];
    let second_wallet = &wallets[1];

    // Setup Predciate
    let hex_predicate_address: &str =
        "0xe5d0a6dbd36c76a091d21e5281c98a0994f6c6b0bc04793532daf4d5b3594743";
    let predicate_address =
        Address::from_str(hex_predicate_address).expect("failed to create Address from string");
    let predicate_bech32_address = Bech32Address::from(predicate_address);
    let predicate_data = AuthPredicateEncoder::default()
        .encode_data(predicate_bech32_address)
        .unwrap();
    let predicate: Predicate =
        Predicate::load_from("test_artifacts/auth_predicate/out/release/auth_predicate.bin")
            .unwrap()
            .with_provider(first_wallet.try_provider().unwrap().clone())
            .with_data(predicate_data);

    // If this test fails, it can be the predicate address
    // Uncomment the next line, get the predicate address and update above.
    // dbg!(&predicate);

    // Next, we lock some assets in this predicate using the first wallet:
    // First wallet transfers amount to predicate.
    first_wallet
        .transfer(predicate.address(), 500, asset_id, TxPolicies::default())
        .await
        .unwrap();

    // Check predicate balance.
    let balance = predicate
        .get_asset_balance(&AssetId::default())
        .await
        .unwrap();
    assert_eq!(balance, 500);

    // Then we can transfer assets owned by the predicate via the Account trait:
    let amount_to_unlock = 500;

    // Will transfer if the correct predicate address is passed as an argument to the predicate
    predicate
        .transfer(
            second_wallet.address(),
            amount_to_unlock,
            asset_id,
            TxPolicies::default(),
        )
        .await
        .unwrap();

    // Predicate balance is zero.
    let balance = predicate
        .get_asset_balance(&AssetId::default())
        .await
        .unwrap();
    assert_eq!(balance, 0);

    // Second wallet balance is updated.
    let balance = second_wallet
        .get_asset_balance(&AssetId::default())
        .await
        .unwrap();
    assert_eq!(balance, 1500);
}

#[tokio::test]
#[should_panic]
async fn when_incorrect_predicate_address_passed() {
    // Setup Wallets
    let asset_id = AssetId::default();
    let wallets_config = WalletsConfig::new_multiple_assets(
        2,
        vec![AssetConfig {
            id: asset_id,
            num_coins: 1,
            coin_amount: 1_000,
        }],
    );
    let wallets = &launch_custom_provider_and_get_wallets(wallets_config, None, None)
        .await
        .unwrap();
    let first_wallet = &wallets[0];
    let second_wallet = &wallets[1];

    // Setup Predciate with incorrect address
    let hex_predicate_address: &str =
        "0x36bf4bd40f2a3b3db595ef8fd8b21dbe9e6c0dd7b419b4413ff6b584ce7da5d7";
    let predicate_address =
        Address::from_str(hex_predicate_address).expect("failed to create Address from string");
    let predicate_data = AuthPredicateEncoder::default()
        .encode_data(Bech32Address::from(predicate_address))
        .unwrap();
    let predicate: Predicate =
        Predicate::load_from("test_artifacts/auth_predicate/out/release/auth_predicate.bin")
            .unwrap()
            .with_provider(first_wallet.try_provider().unwrap().clone())
            .with_data(predicate_data);

    // Next, we lock some assets in this predicate using the first wallet:
    // First wallet transfers amount to predicate.
    first_wallet
        .transfer(predicate.address(), 500, asset_id, TxPolicies::default())
        .await
        .unwrap();

    // Check predicate balance.
    let balance = predicate
        .get_asset_balance(&AssetId::default())
        .await
        .unwrap();
    assert_eq!(balance, 500);

    // Then we can transfer assets owned by the predicate via the Account trait:
    let amount_to_unlock = 500;

    // Will should fail to transfer
    predicate
        .transfer(
            second_wallet.address(),
            amount_to_unlock,
            asset_id,
            TxPolicies::default(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn can_get_predicate_address_in_message() {
    // Setup Predciate address
    let hex_predicate_address: &str =
        "0xe5d0a6dbd36c76a091d21e5281c98a0994f6c6b0bc04793532daf4d5b3594743";
    let predicate_address =
        Address::from_str(hex_predicate_address).expect("failed to create Address from string");
    let predicate_bech32_address = Bech32Address::from(predicate_address);

    // Setup message
    let message_amount = 1;
    let message = Message {
        sender: Bech32Address::default(),
        recipient: predicate_bech32_address.clone(),
        nonce: 0.into(),
        amount: message_amount,
        data: vec![],
        da_height: 0,
        status: MessageStatus::Unspent,
    };
    let mut message_vec: Vec<Message> = Vec::new();
    message_vec.push(message);

    // Setup Coin
    let coin_amount = 0;
    let coin = Coin {
        owner: predicate_bech32_address.clone(),
        utxo_id: UtxoId::new(Bytes32::zeroed(), 0),
        amount: coin_amount,
        asset_id: AssetId::default(),
        status: CoinStatus::Unspent,
        block_created: Default::default(),
    };
    let mut coin_vec: Vec<Coin> = Vec::new();
    coin_vec.push(coin);

    let mut wallet = WalletUnlocked::new_random(None);
    let provider = setup_test_provider(coin_vec, message_vec, None, None)
        .await
        .unwrap();
    wallet.set_provider(provider.clone());

    // Setup Predciate
    let predicate_data = AuthPredicateEncoder::default()
        .encode_data(predicate_bech32_address)
        .unwrap();
    let predicate: Predicate =
        Predicate::load_from("test_artifacts/auth_predicate/out/release/auth_predicate.bin")
            .unwrap()
            .with_provider(wallet.try_provider().unwrap().clone())
            .with_data(predicate_data);

    // If this test fails, it can be the predicate address
    // Uncomment the next line, get the predicate address and update above.
    // dbg!(&predicate);

    // Check predicate balance.
    let balance = predicate
        .get_asset_balance(&AssetId::default())
        .await
        .unwrap();
    assert_eq!(balance, message_amount);

    // Spend the message
    predicate
        .transfer(
            wallet.address(),
            message_amount,
            AssetId::default(),
            TxPolicies::default(),
        )
        .await
        .unwrap();

    // The predicate has spent the funds
    let predicate_balance = predicate
        .get_asset_balance(&AssetId::default())
        .await
        .unwrap();
    assert_eq!(predicate_balance, 0);

    // Funds were transferred
    let wallet_balance = wallet.get_asset_balance(&AssetId::default()).await.unwrap();
    assert_eq!(wallet_balance, message_amount);
}
