use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::util::bip32::{ChildNumber, DerivationPath};
use bdk::bitcoin::Address;
use bdk::blockchain::{Blockchain, ElectrumBlockchain};
use bdk::database::{AnyDatabase, MemoryDatabase};
use bdk::electrum_client::Client;
use bdk::keys::bip39::Mnemonic;
use bdk::keys::DerivableKey;
use bdk::keys::ExtendedKey;
use bdk::wallet::AddressIndex::New;
use bdk::{Error, FeeRate, SignOptions, SyncOptions, Wallet};
use chrono::NaiveDateTime;
use egui_extras::RetainedImage;
use num_format::{Locale, ToFormattedString};
use qrcode_generator::QrCodeEcc;
use std::rc::Rc;
use std::str::FromStr;
// use liquid_rpc::*;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct WalletApp {
    #[serde(skip)]
    image: RetainedImage,
    #[serde(skip)]
    address: String,
    #[serde(skip)]
    send_to: String,
    #[serde(skip)]
    mnemonic: String,
    #[serde(skip)]
    amount: u64,
    #[serde(skip)]
    show: bool,
    #[serde(skip)]
    wallet: Rc<Wallet<AnyDatabase>>,
    #[serde(skip)]
    show_send: bool,
    spendable: u64,
}

impl Default for WalletApp {
    fn default() -> Self {
        // let c = liquid_rpc::CLient::newQ;

        let secp = Secp256k1::new();
        // let mnemonic: GeneratedKey<_, miniscript::BareCtx> =
        //     Mnemonic::generate((WordCount::Words12, Language::English))
        //         .map_err(|_| bdk::Error::Generic("Mnemonic generation error".to_string()))
        //         .unwrap();
        // let _mnemonic = mnemonic.into_key();
        let phrase = "all all all all all all all all all all all all".to_string();
        let mnemonic: Mnemonic = phrase.parse().unwrap();
        let xkey: ExtendedKey = (
            mnemonic.clone(),
            Some("cKwshpAqpkxtfxHXFRGLsnfqHWViDu".to_string()),
        )
            .into_extended_key()
            .unwrap();
        let xprv = xkey
            .into_xprv(bdk::bitcoin::Network::Testnet)
            .ok_or_else(|| {
                Error::Generic("Privatekey info not found (should not happen)".to_string())
            })
            .unwrap();
        let xkey: ExtendedKey = (
            mnemonic.clone(),
            Some("cKwshpAqpkxtfxHXFRGLsnfqHWViDu".to_string()),
        )
            .into_extended_key()
            .unwrap();
        let _xpub = xkey.into_xpub(bdk::bitcoin::Network::Testnet, &secp);
        let phrase = mnemonic
            .word_iter()
            .fold("".to_string(), |phrase, w| phrase + w + " ")
            .trim()
            .to_string();
        let coin_type = 1;
        let base_path = DerivationPath::from_str("m/84'").unwrap();
        let account_number = 0;
        let derivation_path = base_path.extend(&[
            ChildNumber::from_hardened_idx(coin_type).unwrap(),
            ChildNumber::from_hardened_idx(account_number).unwrap(),
        ]);
        let descriptor = bdk::descriptor!(wpkh((
            xprv,
            derivation_path.extend(&[ChildNumber::Normal { index: 0 }])
        )))
        .unwrap();
        let wallet = Wallet::new(
            descriptor,
            None,
            bdk::bitcoin::Network::Testnet,
            AnyDatabase::Memory(MemoryDatabase::new()),
        )
        .unwrap();
        let wallet = Rc::new(wallet);
        let client = Client::new("ssl://electrum.blockstream.info:60002").unwrap();
        let blockchain = ElectrumBlockchain::from(client);
        wallet.sync(&blockchain, SyncOptions::default()).unwrap();
        let spendable = wallet.get_balance().unwrap().get_spendable();
        let qr = qrcode_generator::to_png_to_vec("", QrCodeEcc::Medium, 300).unwrap();
        Self {
            image: RetainedImage::from_image_bytes("default self", qr.as_slice()).unwrap(),
            mnemonic: phrase,
            address: String::new(),
            wallet: wallet,
            amount: 1000,
            show: false,
            send_to: String::new(),
            show_send: false,
            spendable: spendable,
        }
    }
}

impl WalletApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }

    fn update_image(&self, image: egui_extras::RetainedImage) -> Self {
        Self {
            image: image,
            address: self.address.clone(),
            mnemonic: self.mnemonic.clone(),
            wallet: self.wallet.clone(),
            amount: self.amount.clone(),
            show: self.show,
            send_to: self.send_to.clone(),
            show_send: self.show_send,
            spendable: self.spendable,
        }
    }
}

impl eframe::App for WalletApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("id_source")
            .default_width(350.)
            .show(ctx, |ui| {
                egui::TopBottomPanel::top("send_receive").show_inside(ui, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.heading("Send");
                        ui.label(format!(
                            "spendable: {} sats",
                            self.spendable.to_formatted_string(&Locale::en).to_string()
                        ));
                        if ui.button("sync wallet to blockchain").clicked() {
                            let client =
                                Client::new("ssl://electrum.blockstream.info:60002").unwrap();
                            let blockchain = ElectrumBlockchain::from(client);
                            self.wallet
                                .sync(&blockchain, SyncOptions::default())
                                .unwrap();
                            self.spendable = self.wallet.get_balance().unwrap().get_spendable();
                            println!(
                                "spendable {} sats",
                                self.spendable.to_formatted_string(&Locale::en)
                            );
                        }
                        ui.label("amount");
                        let min = 600 as u64;
                        let s =
                            egui::Slider::new(&mut self.amount, min..=self.spendable).text("sats");
                        ui.add(s);
                        ui.label("send sats to");
                        ui.text_edit_singleline(&mut self.send_to);
                        if ui.button("make tx").clicked() {
                            self.show_send = true;
                        }
                        if self.show_send {
                            egui::Window::new("Send?").show(ctx, |ui| {
                                println!("amount {}", self.amount);
                                let send_to: Address = self.send_to.clone().parse().unwrap();
                                let mut builder = self.wallet.build_tx();
                                let (mut psbt, _details) = {
                                    builder
                                        .add_recipient(send_to.script_pubkey(), self.amount)
                                        .fee_rate(FeeRate::from_sat_per_vb(5.0));
                                    builder.finish().unwrap()
                                };
                                ui.label(psbt.to_string());
                                if ui.button("Send?").clicked() {
                                    self.wallet.sign(&mut psbt, SignOptions::default()).unwrap();
                                    println!("{}", psbt.clone().extract_tx().txid());
                                    let client =
                                        Client::new("ssl://electrum.blockstream.info:60002")
                                            .unwrap();
                                    let blockchain = ElectrumBlockchain::from(client);
                                    blockchain.broadcast(&psbt.extract_tx()).unwrap();
                                    self.show_send = false;
                                }
                            });
                        };
                    });
                    ui.add_space(25.);
                    ui.vertical_centered_justified(|ui| {
                        ui.heading("Receive");
                        if ui.button("Fresh Address").clicked() {
                            self.show = true;
                            let address = self.wallet.get_address(New).unwrap().address.to_string();
                            self.address = address.clone();
                            let qr = qrcode_generator::to_png_to_vec(
                                address.to_ascii_uppercase().as_bytes(),
                                QrCodeEcc::High,
                                300,
                            )
                            .unwrap();
                            let img = egui_extras::image::RetainedImage::from_image_bytes(
                                address,
                                qr.as_slice(),
                            )
                            .unwrap();
                            self.update_image(img);
                            self.image.show(ui);
                        }
                        if self.show {
                            self.image.show(ui);
                            if ui.button(self.address.clone()).clicked() {
                                ui.output().copied_text = self.address.clone();
                                self.show = false;
                            };
                        }
                    });
                    ui.end_row();
                });
                egui::Window::new("Mnemonic")
                    .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(0., 0.))
                    .open(&mut true)
                    .collapsible(true)
                    .show(ctx, |ui| {
                        ui.vertical_centered_justified(|ui| {
                            ui.text_edit_singleline(&mut self.mnemonic);
                        });
                    });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.group(|ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.heading("History");
                    let mut txs = self.wallet.list_transactions(true).unwrap();
                    txs.sort_by(|a, b| {
                        b.confirmation_time
                            .as_ref()
                            .map(|t| t.height)
                            .cmp(&a.confirmation_time.as_ref().map(|t| t.height))
                    });
                    egui::Grid::new("tx history").striped(true).show(ui, |ui| {
                        ui.label("txid");
                        ui.label("sats received");
                        ui.label("sats sent");
                        ui.label("fee");
                        ui.label("date");
                        ui.end_row();
                        txs.iter().for_each(|tx| {
                            let txid = tx.txid.to_string();
                            let ts = match tx.confirmation_time.clone() {
                                Some(t) => t.timestamp as i64,
                                None => 1230984932,
                            };
                            let ts = NaiveDateTime::from_timestamp(ts, 0).format("%b %-d, %Y");
                            if ui.button(txid.clone()).clicked() {
                                ui.output().copied_text = txid;
                            }
                            if ui
                                .button(tx.received.to_formatted_string(&Locale::en))
                                .clicked()
                            {
                                ui.output().copied_text = tx.received.to_string();
                            }
                            if ui
                                .button(tx.sent.to_formatted_string(&Locale::en))
                                .clicked()
                            {
                                ui.output().copied_text = tx.sent.to_string();
                            }
                            if ui
                                .button(tx.fee.unwrap().to_formatted_string(&Locale::en))
                                .clicked()
                            {
                                ui.output().copied_text = tx.fee.unwrap().to_string();
                            }
                            ui.label(ts.to_string());
                            ui.add_space(50.);
                            ui.end_row();
                        });
                    });
                });
            });
        });
    }
}
