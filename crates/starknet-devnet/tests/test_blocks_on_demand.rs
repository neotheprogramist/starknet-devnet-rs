pub mod common;

mod blocks_on_demand_tests {
    use std::sync::Arc;

    use serde_json::json;
    use starknet_core::constants::CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH;
    use starknet_rs_accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::{
        BlockId, BlockStatus, BlockTag, FieldElement, FunctionCall, MaybePendingStateUpdate,
    };
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::Provider;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::{
        assert_tx_successful, get_contract_balance, get_contract_balance_by_block_id,
        get_simple_contract_in_sierra_and_compiled_class_hash,
    };

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    async fn assert_pending_state_update(devnet: &BackgroundDevnet) {
        let pending_state_update = &devnet
            .json_rpc_client
            .get_state_update(BlockId::Tag(BlockTag::Pending))
            .await
            .unwrap();

        assert!(matches!(pending_state_update, MaybePendingStateUpdate::PendingUpdate(_)));
    }

    async fn assert_latest_state_update(devnet: &BackgroundDevnet) {
        let latest_state_update =
            &devnet.json_rpc_client.get_state_update(BlockId::Tag(BlockTag::Latest)).await.unwrap();

        assert!(matches!(latest_state_update, MaybePendingStateUpdate::Update(_)));
    }

    async fn assert_latest_block_with_tx_hashes(
        devnet: &BackgroundDevnet,
        block_number: u64,
        transactions: Vec<FieldElement>,
    ) {
        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        assert_eq!(latest_block.block_number, block_number);
        assert_eq!(transactions, latest_block.transactions);
        assert_eq!(latest_block.status, BlockStatus::AcceptedOnL2);

        for tx_hash in latest_block.transactions {
            assert_tx_successful(&tx_hash, &devnet.json_rpc_client).await;
        }
    }

    async fn assert_pending_block_with_tx_hashes(devnet: &BackgroundDevnet, tx_count: usize) {
        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();

        assert_eq!(pending_block.transactions.len(), tx_count);

        for tx_hash in pending_block.transactions {
            assert_tx_successful(&tx_hash, &devnet.json_rpc_client).await;
        }
    }

    async fn assert_latest_block_with_txs(
        devnet: &BackgroundDevnet,
        block_number: u64,
        tx_count: usize,
    ) {
        let latest_block = devnet.get_latest_block_with_txs().await.unwrap();

        assert_eq!(latest_block.block_number, block_number);
        assert_eq!(latest_block.status, BlockStatus::AcceptedOnL2);
        assert_eq!(latest_block.transactions.len(), tx_count);

        for tx in latest_block.transactions {
            assert_tx_successful(tx.transaction_hash(), &devnet.json_rpc_client).await;
        }
    }

    async fn assert_pending_block_with_txs(devnet: &BackgroundDevnet, tx_count: usize) {
        let pending_block = devnet.get_pending_block_with_txs().await.unwrap();

        assert_eq!(pending_block.transactions.len(), tx_count);

        for tx in pending_block.transactions {
            assert_tx_successful(tx.transaction_hash(), &devnet.json_rpc_client).await;
        }
    }

    async fn assert_latest_block_with_receipts(
        devnet: &BackgroundDevnet,
        block_number: u64,
        tx_count: usize,
    ) {
        let latest_block = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithReceipts",
                json!(    {
                    "block_id": "latest"
                }),
            )
            .await["result"];

        assert_eq!(latest_block["transactions"].as_array().unwrap().len(), tx_count);
        assert_eq!(latest_block["block_number"], block_number);
        assert_eq!(latest_block["status"], "ACCEPTED_ON_L2");

        for tx in latest_block["transactions"].as_array().unwrap() {
            assert_tx_successful(
                &FieldElement::from_hex_be(tx["receipt"]["transaction_hash"].as_str().unwrap())
                    .unwrap(),
                &devnet.json_rpc_client,
            )
            .await;
        }
    }

    async fn assert_pending_block_with_receipts(devnet: &BackgroundDevnet, tx_count: usize) {
        let pending_block = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithReceipts",
                json!(    {
                    "block_id": "pending"
                }),
            )
            .await["result"];

        assert!(pending_block["status"].is_null());
        assert_eq!(pending_block["transactions"].as_array().unwrap().len(), tx_count);

        for tx in pending_block["transactions"].as_array().unwrap() {
            assert_tx_successful(
                &FieldElement::from_hex_be(tx["receipt"]["transaction_hash"].as_str().unwrap())
                    .unwrap(),
                &devnet.json_rpc_client,
            )
            .await;
        }
    }

    async fn assert_balance(devnet: &BackgroundDevnet, expected: FieldElement, tag: BlockTag) {
        let balance = devnet
            .get_balance_by_tag(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
                tag,
            )
            .await
            .unwrap();
        assert_eq!(balance, expected);
    }

    async fn assert_get_nonce(devnet: &BackgroundDevnet) {
        let (_, account_address) = devnet.get_first_predeployed_account().await;

        let pending_block_nonce = devnet
            .json_rpc_client
            .get_nonce(BlockId::Tag(BlockTag::Pending), account_address)
            .await
            .unwrap();
        assert_eq!(pending_block_nonce, FieldElement::ZERO);

        let latest_block_nonce = devnet
            .json_rpc_client
            .get_nonce(BlockId::Tag(BlockTag::Latest), account_address)
            .await
            .unwrap();
        assert_eq!(latest_block_nonce, FieldElement::ZERO);
    }

    async fn assert_get_storage_at(devnet: &BackgroundDevnet) {
        let (_, account_address) = devnet.get_first_predeployed_account().await;
        let key = FieldElement::ZERO;

        let pending_block_storage = devnet
            .json_rpc_client
            .get_storage_at(account_address, key, BlockId::Tag(BlockTag::Pending))
            .await
            .unwrap();
        assert_eq!(pending_block_storage, FieldElement::ZERO);

        let latest_block_storage = devnet
            .json_rpc_client
            .get_storage_at(account_address, key, BlockId::Tag(BlockTag::Latest))
            .await
            .unwrap();
        assert_eq!(latest_block_storage, FieldElement::ZERO);
    }

    async fn assert_get_class_hash_at(devnet: &BackgroundDevnet) {
        let (_, account_address) = devnet.get_first_predeployed_account().await;

        let pending_block_class_hash = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Pending), account_address)
            .await
            .unwrap();
        assert_eq!(
            pending_block_class_hash,
            FieldElement::from_hex_be(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap()
        );

        let latest_block_class_hash = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Latest), account_address)
            .await
            .unwrap();
        assert_eq!(
            latest_block_class_hash,
            FieldElement::from_hex_be(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap()
        );
    }

    #[tokio::test]
    async fn normal_mode_states_and_blocks() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let tx_count = 5;
        let mut tx_hashes = Vec::new();
        for _ in 0..tx_count {
            let mint_hash = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
            tx_hashes.push(mint_hash);
        }

        assert_balance(&devnet, FieldElement::from(tx_count * DUMMY_AMOUNT), BlockTag::Pending)
            .await;
        assert_balance(&devnet, FieldElement::from(tx_count * DUMMY_AMOUNT), BlockTag::Latest)
            .await;

        assert_pending_block_with_tx_hashes(&devnet, 0).await;
        assert_pending_block_with_txs(&devnet, 0).await;
        assert_pending_block_with_receipts(&devnet, 0).await;

        assert_latest_block_with_tx_hashes(&devnet, 5, vec![tx_hashes.last().copied().unwrap()])
            .await;
        assert_latest_block_with_txs(&devnet, 5, 1).await;
        assert_latest_block_with_receipts(&devnet, 5, 1).await;

        assert_pending_state_update(&devnet).await;
        assert_latest_state_update(&devnet).await;
    }

    #[tokio::test]
    async fn blocks_on_demand_states_and_blocks() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"]).await.unwrap();

        let tx_count = 5_usize;
        let mut tx_hashes = Vec::new();
        for _ in 0..tx_count {
            let mint_hash = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
            tx_hashes.push(mint_hash);
        }

        assert_balance(
            &devnet,
            FieldElement::from(tx_count * DUMMY_AMOUNT as usize),
            BlockTag::Pending,
        )
        .await;
        assert_balance(&devnet, FieldElement::from(0_u128), BlockTag::Latest).await;

        assert_pending_block_with_tx_hashes(&devnet, tx_count).await;
        assert_pending_block_with_txs(&devnet, tx_count).await;
        assert_pending_block_with_receipts(&devnet, tx_count).await;

        assert_latest_block_with_tx_hashes(&devnet, 0, vec![]).await;
        assert_latest_block_with_txs(&devnet, 0, 0).await;
        assert_latest_block_with_receipts(&devnet, 0, 0).await;

        // create new block from pending block
        devnet.create_block().await.unwrap();

        assert_balance(
            &devnet,
            FieldElement::from(tx_count * DUMMY_AMOUNT as usize),
            BlockTag::Pending,
        )
        .await;
        assert_balance(
            &devnet,
            FieldElement::from(tx_count * DUMMY_AMOUNT as usize),
            BlockTag::Latest,
        )
        .await;

        assert_pending_block_with_tx_hashes(&devnet, 0).await;
        assert_pending_block_with_txs(&devnet, 0).await;
        assert_pending_block_with_receipts(&devnet, 0).await;

        assert_latest_block_with_tx_hashes(&devnet, 1, tx_hashes).await;
        assert_latest_block_with_txs(&devnet, 1, tx_count).await;
        assert_latest_block_with_receipts(&devnet, 1, tx_count).await;

        assert_pending_state_update(&devnet).await;
        assert_latest_state_update(&devnet).await;
    }

    #[tokio::test]
    async fn blocks_on_demand_invoke_and_call() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"]).await.unwrap();

        let mut tx_hashes = Vec::new();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer.clone(),
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        ));

        let (contract_class, casm_class_hash) =
            get_simple_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(contract_class), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .nonce(FieldElement::ZERO)
            .send()
            .await
            .unwrap();

        tx_hashes.push(declaration_result.transaction_hash);

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());
        let initial_value = FieldElement::from(10_u32);
        let ctor_args = vec![initial_value];
        let deploy_result = contract_factory
            .deploy(ctor_args.clone(), FieldElement::ZERO, false)
            .max_fee(FieldElement::from(1e18 as u128))
            .nonce(FieldElement::ONE)
            .send()
            .await
            .unwrap();

        tx_hashes.push(deploy_result.transaction_hash);

        // generate the address of the newly deployed contract
        let contract_address = get_udc_deployed_address(
            FieldElement::ZERO,
            declaration_result.class_hash,
            &starknet_rs_core::utils::UdcUniqueness::NotUnique,
            &ctor_args,
        );

        let increment = FieldElement::from(5_u32);
        let contract_invoke = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("increase_balance").unwrap(),
            calldata: vec![increment, FieldElement::ZERO],
        }];
        let increment_count = 2;
        for i in 1..=increment_count {
            let invoke_result = predeployed_account
                .execute(contract_invoke.clone())
                .max_fee(FieldElement::from(1e18 as u128))
                .nonce(FieldElement::from(i + 1_u128))
                .send()
                .await
                .unwrap();

            assert_tx_successful(&invoke_result.transaction_hash, &devnet.json_rpc_client).await;

            tx_hashes.push(invoke_result.transaction_hash);
        }

        let expected_balance = initial_value + (increment * FieldElement::from(increment_count));

        assert_eq!(
            get_contract_balance_by_block_id(
                &devnet,
                contract_address,
                BlockId::Tag(BlockTag::Pending)
            )
            .await,
            expected_balance
        );

        let contract_call = FunctionCall {
            contract_address,
            entry_point_selector: get_selector_from_name("get_balance").unwrap(),
            calldata: vec![],
        };
        let latest_block_balance =
            devnet.json_rpc_client.call(contract_call, BlockId::Tag(BlockTag::Latest)).await;
        assert!(latest_block_balance.is_err());

        devnet.create_block().await.unwrap();

        assert_latest_block_with_tx_hashes(&devnet, 1, tx_hashes).await;
        assert_eq!(
            get_contract_balance_by_block_id(
                &devnet,
                contract_address,
                BlockId::Tag(BlockTag::Pending)
            )
            .await,
            expected_balance
        );
        assert_eq!(get_contract_balance(&devnet, contract_address).await, expected_balance);
    }

    #[tokio::test]
    async fn get_nonce_of_first_predeployed_account_normal_mode() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        assert_get_nonce(&devnet).await;
    }

    #[tokio::test]
    async fn get_nonce_of_first_predeployed_account_block_on_demand() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"]).await.unwrap();

        assert_get_nonce(&devnet).await;
    }

    #[tokio::test]
    async fn get_storage_at_normal_mode() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        assert_get_storage_at(&devnet).await;
    }

    #[tokio::test]
    async fn get_storage_at_block_on_demand() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"]).await.unwrap();

        assert_get_storage_at(&devnet).await;
    }

    #[tokio::test]
    async fn get_class_hash_at_normal_mode() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        assert_get_class_hash_at(&devnet).await;
    }

    #[tokio::test]
    async fn get_class_hash_at_block_on_demand() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"]).await.unwrap();

        assert_get_class_hash_at(&devnet).await;
    }
}
