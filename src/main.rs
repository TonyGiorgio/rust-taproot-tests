use std::str::FromStr;

use bdk::blockchain::rpc::{Auth, RpcBlockchain, RpcConfig};
use bdk::blockchain::{Blockchain, ConfigurableBlockchain};
use bdk::database::MemoryDatabase;
use bdk::signer::TapLeavesOptions;
use bdk::wallet::AddressIndex::New;
use bdk::{FeeRate, SignOptions, SyncOptions, Wallet};
use bitcoin::consensus::encode;
use bitcoin::hash_types::Txid;
use bitcoin::secp256k1::{rand, PublicKey, Secp256k1, SecretKey};
use bitcoin::util::psbt::PartiallySignedTransaction as Psbt;
use bitcoin::util::taproot::TapLeafHash;
use bitcoin::{OutPoint, Script, Transaction, TxIn, TxOut, Witness};
use bitcoincore_rpc::{Client, RpcApi};
use std::io;

fn main() -> Result<(), bdk::Error> {
    let secp_unspendable = Secp256k1::new();
    let secret_key_unspendable = SecretKey::new(&mut rand::thread_rng());
    let private_key_unspendable =
        bitcoin::PrivateKey::new(secret_key_unspendable, bitcoin::Network::Regtest);
    let public_key_unspendable =
        bitcoin::PublicKey::from_private_key(&secp_unspendable, &private_key_unspendable);

    let secp = Secp256k1::new();
    let secret_key = SecretKey::new(&mut rand::thread_rng());
    let private_key = bitcoin::PrivateKey::new(secret_key, bitcoin::Network::Regtest);
    let _public_key = bitcoin::PublicKey::from_private_key(&secp, &private_key);

    let secp2 = Secp256k1::new();
    let secret_key2 = SecretKey::new(&mut rand::thread_rng());
    let private_key2 = bitcoin::PrivateKey::new(secret_key2, bitcoin::Network::Regtest);
    let _public_key2 = bitcoin::PublicKey::from_private_key(&secp2, &private_key2);

    let (descriptor, _key_map, _networks) = bdk::descriptor!(tr(
        public_key_unspendable,
        multi_a(1, private_key, private_key2)
    ))?;
    let wallet = Wallet::new(
        descriptor,
        None,
        bitcoin::Network::Regtest,
        MemoryDatabase::default(),
    )?;

    println!(
        "desc: {}",
        wallet
            .public_descriptor(bdk::KeychainKind::External)?
            .unwrap()
    );

    println!("Address #0: {}", wallet.get_address(New)?);

    println!("Press enter when you sent transaction and mined...");
    let mut input_string = String::new();
    input_string.clear();
    io::stdin().read_line(&mut input_string).unwrap();

    // sync the blockchain afterwards
    let config = RpcConfig {
        url: "127.0.0.1:18443".to_string(),
        auth: Auth::UserPass {
            username: String::from("polaruser"),
            password: String::from("polarpass"),
        },
        network: bdk::bitcoin::Network::Regtest,
        wallet_name: "wallet_name".to_string(),
        sync_params: None,
    };
    let blockchain = RpcBlockchain::from_config(&config).unwrap();
    wallet.sync(&blockchain, SyncOptions::default())?;
    println!("balance: {}", wallet.get_balance()?);

    let send_to = wallet.get_address(New)?;
    let (mut psbt, details) = {
        let mut builder = wallet.build_tx();
        builder
            .add_recipient(send_to.script_pubkey(), 50_000)
            .enable_rbf()
            .fee_rate(FeeRate::from_sat_per_vb(5.0));
        builder.finish()?
    };

    /*
    let script_leaves: Vec<_> = psbt.inputs[0]
        .tap_scripts
        .clone()
        .values()
        .map(|(script, version)| TapLeafHash::from_script(script, *version))
        .collect();
    */
    let serialized_tx = encode::serialize_hex(&psbt);
    println!("about to sign transaction: {}", serialized_tx);
    let finalized = wallet.sign(&mut psbt, SignOptions::default())?;
    if !finalized {
        println!("did not finalize transaction");
    }
    let serialized_tx = encode::serialize_hex(&psbt);
    println!("about to broadcast transaction: {}", serialized_tx);

    // broadcast
    match blockchain.broadcast(&psbt.extract_tx()) {
        Ok(_) => println!("broadcasted successfully"),
        Err(e) => println!("broadcast error: {}", e),
    };

    Ok(())
}
