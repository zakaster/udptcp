// #![allow(unused)]
use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::mpsc;
use std::time::Duration;

use eframe::egui;

mod network;
use network::Netif;
mod gui;

mod xlogger;
use xlogger::Xlogger;

mod tcp;
mod udp;

/// rust egui udp / tcp tester program
///
/// running the app
///     run multiple / stop multiple
///         use ./start_many and ./stop_many
///
///     run with cargo watch
///         cargo watch -w src/main.rs -x 'run'
///
struct App {
    netif_vec: Vec<Netif>,
    netif_selected: usize, // index in dropdown

    // new - split tcp and udp
    local_ip: String,
    local_port_udp: String,
    broadcast_ip_udp: String,
    broadcast_ip_manual_udp: String,
    remote_ip_udp: String,
    remote_port_udp: String,
    local_port_tcp_server: String,
    local_port_tcp_client: String,
    remote_ip_tcpserver: String,
    remote_port_tcpserver: String,

    // udp
    udp: udp::Udp,
    udp_bc: bool,

    tcpserver: tcp::TcpServer,
    tcpclient: tcp::TcpClient,

    // since connected clients are in a vec (ordered)
    // using position (usize) to keep tracking them is easy to do
    // but with a downside when the collection gets changed
    // eg [a, b, c] are current connected clients, and selected = 1
    // when first one `a` gets removed, now `c` is now on position 1
    // so b is deselected and c is now selected
    selected_clients: HashSet<SocketAddr>,

    msg: String,
    log: Vec<String>,
    logrx: mpsc::Receiver<String>,

    // settings
    dark_mode: bool,
}

impl App {
    fn new() -> Self {
        let logrx = Xlogger::init();
        log::info!(">>> starting app {} <<<", chrono::Local::now());

        let mut app = Self {
            netif_vec: Netif::get_local_netif(),
            netif_selected: 0,

            local_ip: String::default(),
            local_port_udp: String::default(),
            broadcast_ip_udp: String::default(),
            broadcast_ip_manual_udp: String::default(),
            remote_ip_udp: String::default(),
            remote_port_udp: String::default(),
            local_port_tcp_server: String::default(),
            local_port_tcp_client: String::default(),
            remote_ip_tcpserver: String::default(),
            remote_port_tcpserver: String::default(),

            udp: udp::Udp::default(),
            udp_bc: false,

            // tcp_server_mode: false,
            tcpserver: tcp::TcpServer::default(),
            selected_clients: HashSet::new(),
            tcpclient: tcp::TcpClient::default(),

            msg: String::new(),
            log: vec![],
            logrx,

            // settings
            dark_mode: false,
        };

        app.apply_netif_selection(0);
        app
    }

    /// helper
    fn apply_netif_selection(&mut self, index: usize) {
        if let Some(netif) = self.netif_vec.get(index) {
            self.netif_selected = index;

            self.local_ip = netif.ip.to_string();
            self.local_port_udp = "0".to_string();
            self.broadcast_ip_udp = netif.bc.map(|ip| ip.to_string()).unwrap_or_default();
            self.broadcast_ip_manual_udp = String::default();
            self.remote_ip_udp = netif.remote_ip_template();
            self.remote_port_udp = String::default();
            self.local_port_tcp_server = "0".to_string();
            self.local_port_tcp_client = "0".to_string();
            self.remote_ip_tcpserver = netif.remote_ip_template();
            self.remote_port_tcpserver = String::default();
        }
    }

    fn is_tcp_running(&self) -> bool {
        self.tcpserver.is_up() || self.tcpclient.is_up()
    }

    /// we need to constantly poll from the log channel
    /// to be able to show them in GUI
    fn update_logs(&mut self) {
        while let Ok(line) = self.logrx.try_recv() {
            self.log.push(line);
        }
    }

    /// used by broadcast toggle
    /// for loopback and unspecific nics there are no bc feature
    fn local_is_loopback_or_unspecified(&self) -> bool {
        self.local_ip
            .parse::<Ipv4Addr>()
            .map(|ip| ip.is_loopback() || ip.is_unspecified())
            .unwrap_or(false)
    }

    // optional
    #[allow(dead_code)]
    #[deprecated = "old, not that useful"]
    fn render_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("tool_bar")
            .default_height(100.0)
            .show(ctx, |ui| {
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label("Dark mode");
                    gui::toggle_ui(ui, &mut self.dark_mode);

                    ui.separator();

                    // no bc for localhost and unspecified
                    ui.label("UDP broadcast");
                    if ui
                        .add_enabled(
                            !self.local_is_loopback_or_unspecified(),
                            gui::my_toggle(&mut self.udp_bc),
                        )
                        .clicked()
                    {
                        match self.udp.toggle_broadcast(self.udp_bc) {
                            Ok(()) => log::info!("broadcast set to {}", self.udp_bc),
                            Err(e) => {
                                log::error!(
                                    "failed to set broadcast to {}, err = {e}",
                                    self.udp_bc
                                );

                                // revert in case of failure
                                self.udp_bc = !self.udp_bc;
                            }
                        }
                    }

                    ui.separator();

                    // this toggle should be only enabled when tcp is not running
                    // ui.label("TCP host");
                    // ui.add_enabled(
                    //     !self.is_tcp_running(),
                    //     gui::my_toggle(&mut self.tcp_server_mode),
                    // );

                    ui.separator();

                    if ui
                        .add_sized([80.0, ui.available_height()], egui::Button::new("Clear"))
                        .clicked()
                    {
                        self.log.clear();
                        log::info!("--- reset log {} ---", chrono::Local::now());
                    }

                    ui.separator();

                    if ui.button("test").clicked() {
                        if let Some(viewport) = ctx.input(|i| i.viewport().outer_rect) {
                            println!("--- APP INFO ---");
                            println!("app position: {}", viewport.min); // top-left corner
                            println!("app size: {}", viewport.size());
                            println!("app dark mode: {:?}", ctx.style().visuals.dark_mode);
                            println!("--- END APP INFO ---");
                        }
                    }
                });

                ui.add_space(5.0);
            });
    }

    fn render_local_netif_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("local_netif")
            .default_height(100.0)
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.add_space(5.0);

                let netif_pre = self.netif_selected;
                ui.horizontal(|ui| {
                    if ui.button("⟳").clicked() {
                        self.netif_vec = Netif::get_local_netif();
                        self.apply_netif_selection(netif_pre);
                        log::info!("local netif updated");
                    };

                    // local netif selection combo
                    // not editable if socket connected
                    egui::ComboBox::from_id_salt("combo_netif_local")
                        .selected_text(
                            self.netif_vec
                                .get(self.netif_selected)
                                .map(|iface| iface.to_string())
                                .unwrap_or_default(),
                        )
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for (i, interface) in self.netif_vec.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.netif_selected,
                                    i,
                                    interface.to_string(),
                                );
                            }
                        });

                    // using a unified local port for both tcp and udp would be a
                    // bad idea because when tcp runs as client, the port is ephemeral
                    // if we had udp already connected to 13400, then starts the tcp client
                    // the local port being used will definitely not 13400, so which port
                    // should we display on GUI?
                    // ui.add(egui::TextEdit::singleline(&mut self.local_port).desired_width(80.0))
                    //     .on_hover_text("local port");
                });
                if self.netif_selected != netif_pre {
                    self.apply_netif_selection(self.netif_selected);
                }
            });
    }

    fn render_udp_and_tcp_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("udp_tcp")
            .default_height(170.0)
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.add_space(5.0);
                ui.columns(3, |cui| {
                    // udp column
                    cui[0].group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.set_height(ui.available_height());

                        // centered button (selectable label)
                        let udp_label = if self.udp.is_up() {
                            "UDP Stop"
                        } else {
                            "UDP Start"
                        };
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), 26.0),
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                // Stretch to full width; text appears centered due to layout
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 26.0],
                                        egui::SelectableLabel::new(self.udp.is_up(), udp_label),
                                    )
                                    .clicked()
                                {
                                    // if UDP is not running
                                    if !self.udp.is_up() {
                                        let localsock =
                                            format!("{}:{}", self.local_ip, self.local_port_udp);
                                        match self.udp.connect_and_start(localsock) {
                                            Ok(port) => {
                                                self.local_port_udp = port;
                                            }
                                            Err(e) => {
                                                log::error!("starting UDP failed: {e}");
                                                return;
                                            }
                                        }
                                    } else {
                                        // if UDP is already running, turn it off
                                        self.udp.disconnect();
                                    }
                                }
                            },
                        );
                        ui.add_space(4.0);

                        // todo
                        // reserved a way to be able to disable the ui via a flag
                        // here the flag is not used,
                        ui.add_enabled_ui(true, |ui| {
                            egui::Grid::new("udp_grid")
                                .num_columns(2)
                                .spacing([20.0, 4.0])
                                .striped(false)
                                // .min_col_width(150.0)
                                .show(ui, |ui| {
                                    ui.label("Broadcast");
                                    if ui
                                        .add_enabled(
                                            !self.local_is_loopback_or_unspecified(),
                                            gui::my_toggle(&mut self.udp_bc),
                                        )
                                        .clicked()
                                    {
                                        if let Err(e) = self.udp.toggle_broadcast(self.udp_bc) {
                                            log::error!(
                                                "failed to set broadcast to {}, err = {e}",
                                                self.udp_bc
                                            );

                                            // revert in case of failure
                                            self.udp_bc = !self.udp_bc;
                                        }
                                    }
                                    ui.end_row();

                                    ui.label("Manual Broadcast Address");
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            &mut self.broadcast_ip_manual_udp,
                                        )
                                        .desired_width(ui.available_width()),
                                    );
                                    ui.end_row();

                                    ui.label("Local Port");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.local_port_udp)
                                            .desired_width(ui.available_width()),
                                    );
                                    ui.end_row();

                                    ui.label("Remote Address");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.remote_ip_udp)
                                            .desired_width(ui.available_width()),
                                    );
                                    ui.end_row();

                                    ui.label("Remote Port");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.remote_port_udp)
                                            .desired_width(ui.available_width()),
                                    );
                                    ui.end_row();
                                }); // grid end
                        });
                    });

                    // tcp host column
                    cui[1].group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.set_height(ui.available_height());

                        let tcp_label = if self.tcpserver.is_up() {
                            "TCP Server Stop"
                        } else {
                            "TCP Server Start"
                        };
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), 26.0),
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                // Stretch to full width; text appears centered due to layout
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 26.0],
                                        egui::SelectableLabel::new(
                                            self.tcpserver.is_up(),
                                            tcp_label,
                                        ),
                                    )
                                    .clicked()
                                {
                                    if self.tcpserver.is_up() {
                                        self.tcpserver.disconnect();
                                    } else {
                                        let sockaddr = format!(
                                            "{}:{}",
                                            self.local_ip, self.local_port_tcp_server
                                        );
                                        if let Some(port) = self.tcpserver.begin(sockaddr) {
                                            self.local_port_tcp_server = port;
                                        }
                                    }
                                }
                            },
                        );
                        ui.add_space(4.0);

                        egui::Grid::new("tcp_server_grid")
                            .num_columns(2)
                            .spacing([20.0, 4.0])
                            .striped(false)
                            // .min_col_width(150.0)
                            .show(ui, |ui| {
                                ui.label("Local Port");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.local_port_tcp_server)
                                        .desired_width(ui.available_width()),
                                );
                                ui.end_row();
                            });

                        ui.add_space(4.0);
                        ui.label("Connected peers...");
                        ui.separator();

                        egui::ScrollArea::vertical()
                            // .auto_shrink([false, false])
                            // .auto_shrink([true, true])
                            // .max_height(20.0)
                            .max_height(ui.available_height())
                            .show(ui, |ui| {
                                // notice in this looping we are modifying the collection
                                for stream_ref in self.tcpserver.clients.iter() {
                                    let Ok(peer_addr) = stream_ref.peer_addr() else {
                                        log::error!(
                                            "unable to retrive peer address from {:?}, skipped",
                                            stream_ref
                                        );
                                        continue;
                                    };

                                    // let selected = self.selected_peers.contains(&peer_addr);
                                    let selected = self.selected_clients.contains(&peer_addr);

                                    let resp = ui.add(egui::SelectableLabel::new(
                                        selected,
                                        peer_addr.to_string(),
                                    ));

                                    // select - diselect
                                    if resp.clicked() {
                                        if selected {
                                            self.selected_clients.remove(&peer_addr);
                                        } else {
                                            self.selected_clients.insert(peer_addr);
                                        }
                                    }

                                    // close connection with double click
                                    if resp.double_clicked() {
                                        self.selected_clients.remove(&peer_addr);
                                        self.tcpserver.close_client(peer_addr);
                                    }

                                    // close with right click popup
                                    resp.context_menu(|ui| {
                                        if ui.button("Close").clicked() {
                                            self.selected_clients.remove(&peer_addr);
                                            self.tcpserver.close_client(peer_addr);
                                            ui.close_menu(); // ensure the menu closes
                                        }
                                    });
                                }
                            });
                    });

                    cui[2].group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.set_height(ui.available_height());

                        let tcp_label = if self.tcpclient.is_up() {
                            "TCP Client Stop"
                        } else {
                            "TCP Client Start"
                        };
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), 26.0),
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                // Stretch to full width; text appears centered due to layout
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 26.0],
                                        egui::SelectableLabel::new(
                                            self.tcpclient.is_up(),
                                            tcp_label,
                                        ),
                                    )
                                    .clicked()
                                {
                                    /***** tcp client code ****/

                                    /***** tcp client code *****/
                                    if self.tcpclient.is_up() {
                                        self.tcpclient.disconnect();
                                    } else {
                                        let sockaddr = format!(
                                            "{}:{}",
                                            self.remote_ip_tcpserver, self.remote_port_tcpserver
                                        );
                                        if let Some(sock) = self.tcpclient.begin(&sockaddr) {
                                            self.local_ip = sock.ip().to_string();
                                            self.local_port_tcp_client = sock.port().to_string();
                                        }
                                    }
                                }
                            },
                        );
                        ui.add_space(4.0);

                        egui::Grid::new("tcp_grid")
                            .num_columns(2)
                            .spacing([20.0, 4.0])
                            .striped(false)
                            // .min_col_width(150.0)
                            .show(ui, |ui| {
                                ui.label("Local Port");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.local_port_tcp_client)
                                        .desired_width(ui.available_width()),
                                );
                                ui.end_row();

                                ui.label("Server Address");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.remote_ip_tcpserver)
                                        .desired_width(ui.available_width()),
                                );
                                ui.end_row();
                                ui.label("Server Port");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.remote_port_tcpserver)
                                        .desired_width(ui.available_width()),
                                );
                                ui.end_row();
                            });
                    });
                });
            });
    }

    // helper method
    fn color_logs(line: &str) -> egui::Color32 {
        if line.contains("SEND") || line.contains("RECV") {
            return egui::Color32::from_rgb(65, 105, 225); // blue
        } else if line.contains("ERR") {
            return egui::Color32::LIGHT_RED;
        } else {
            return egui::Color32::DARK_GRAY;
        }
    }

    // new version
    // abandon the fine control over the font size and coloring
    // add sense to the scrollarea
    fn render_log_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // wrap the scrollarea into a none frame in order to get sense
            let frame_out = egui::Frame::NONE.show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .hscroll(true)
                    .show(ui, |ui| {
                        for line in self.log.iter() {
                            let color = Self::color_logs(&line);

                            // extend() to avoid soft line wrapping
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(line).monospace().color(color),
                                )
                                .extend(),
                            );
                        }
                    });
            });
            // Right-click anywhere in the log area to open the menu
            frame_out.response.context_menu(|ui| {
                if ui.button("Clear log").clicked() {
                    self.log.clear();
                    log::info!("--- reset log {} ---", chrono::Local::now());
                    ui.close_menu();
                }
            });
        });
    }

    #[allow(dead_code)]
    #[deprecated = "earlier approach with no sense"]
    fn render_log_panel_old(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .hscroll(true)
                .show(ui, |ui| {
                    for line in self.log.iter() {
                        let text_color = if ui.visuals().dark_mode {
                            if line.contains("SEND") || line.contains("RECV") {
                                egui::Color32::from_rgb(229, 192, 123) // one Dark Pro yellow
                            } else if line.contains("ERR") {
                                egui::Color32::LIGHT_RED
                            } else {
                                egui::Color32::LIGHT_GRAY
                            }
                        } else {
                            if line.contains("SEND") || line.contains("RECV") {
                                egui::Color32::from_rgb(65, 105, 225) // blue
                            } else if line.contains("ERR") {
                                egui::Color32::LIGHT_RED
                            } else {
                                egui::Color32::DARK_GRAY
                            }
                        };
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(line)
                                    .font(egui::FontId::monospace(
                                        ctx.style().text_styles[&egui::TextStyle::Body].size - 0.5,
                                    ))
                                    .color(text_color),
                            )
                            .extend(),
                        );
                    }
                });
        });
    }

    fn render(&mut self, ctx: &egui::Context) {
        // self.render_toolbar(ctx);
        self.render_local_netif_panel(ctx);

        self.render_udp_and_tcp_panel(ctx);

        // textedit for message to send and a send button
        // this section is enabled only when either udp or tcp is available
        egui::TopBottomPanel::top("message_to_send")
            .default_height(100.0)
            .show(ctx, |ui| {
                ui.add_space(5.0);
                ui.vertical_centered_justified(|ui| {
                    ui.text_edit_singleline(&mut self.msg);
                    ui.add_space(5.0);
                    if ui.button("SEND").clicked() {
                        if self.msg.is_empty() {
                            return;
                        }

                        /*** UDP send handling ***/
                        if self.udp.is_up() {
                            let remote_ip = if !self.udp_bc {
                                self.remote_ip_udp.clone()
                            } else {
                                if !self.broadcast_ip_manual_udp.is_empty() {
                                    self.broadcast_ip_manual_udp.clone()
                                } else {
                                    self.broadcast_ip_udp.clone()
                                }
                            };
                            let remote_sockaddr = format!("{}:{}", remote_ip, self.remote_port_udp);
                            self.udp.send_data_to(&self.msg, &remote_sockaddr)
                        }

                        /* TCP client send handling */
                        if self.tcpclient.is_up() {
                            self.tcpclient.send_data(&self.msg);
                        }

                        /* TCP host send handling */
                        if self.tcpserver.is_up() {
                            // if self.selected_peers_new.is_empty() {
                            //     log::error!("no destination peer selected for TcpServer to send");
                            //     return;
                            // }
                            for stream in self.tcpserver.clients.iter() {
                                let Ok(peer_addr) = stream.peer_addr() else {
                                    log::error!(
                                        "unable to retrive peer address from {:?} when sending, skipped",
                                        stream
                                    );
                                    continue;
                                };
                                if self.selected_clients.contains(&peer_addr) {
                                    // extract the mutable stream
                                    self.tcpserver.send_data(&self.msg, stream);
                                }
                            }
                        }
                    };
                });
                ui.add_space(5.0);
            });

        // log panel
        self.render_log_panel(ctx);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // drive a periodic repaint
        // with this periodic repaint, we dont need the manual
        // repaint inside tcp or udp anymore
        if self.udp.is_up() || self.is_tcp_running() {
            ctx.request_repaint_after(Duration::from_millis(50)); // 20fps
        }

        if self.tcpserver.is_up() {
            self.tcpserver.poll_events();

            // todo optional
            // this part is suggested by gpt but in my testing
            // the app works fine without the block, so disable for now
            // also not verified with the code block enabled
            // let current: HashSet<SocketAddr> = self
            //     .tcpserver
            //     .clients
            //     .iter()
            //     .filter_map(|s| s.peer_addr().ok())
            //     .collect();
            // self.selected_clients.retain(|a| current.contains(a));
        }

        self.update_logs();

        // mode
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        self.render(ctx);
    }
}

/// patch
/// Try to locate a system CJK font cross‑platform and install as fallback for both families.
/// cjk = chinese, japanese, korean
fn install_cjk_fallback(ctx: &egui::Context) {
    use std::{env, fs, path::PathBuf, sync::Arc};

    fn candidates() -> Vec<PathBuf> {
        // Highest priority: explicit override
        let mut paths = env::var_os("UDPTCP_CJK_FONT")
            .map(|p| vec![PathBuf::from(p)])
            .unwrap_or_default();

        // OS-specific guesses
        #[cfg(target_os = "windows")]
        {
            paths.extend(
                [
                    r"C:\Windows\Fonts\msyh.ttc", // Microsoft YaHei
                    r"C:\Windows\Fonts\msyh.ttf",
                    r"C:\Windows\Fonts\msjh.ttc", // Microsoft JhengHei (TC)
                    r"C:\Windows\Fonts\simhei.ttf", // SimHei
                    r"C:\Windows\Fonts\simsun.ttc", // SimSun
                ]
                .into_iter()
                .map(PathBuf::from),
            );
        }
        #[cfg(target_os = "macos")]
        {
            paths.extend(
                [
                    "/System/Library/Fonts/PingFang.ttc",         // PingFang SC
                    "/System/Library/Fonts/STHeiti Light.ttc",    // Heiti
                    "/System/Library/Fonts/Songti.ttc",           // Songti
                    "/System/Library/Fonts/Hiragino Sans GB.ttc", // Some macOS builds
                ]
                .into_iter()
                .map(PathBuf::from),
            );
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            paths.extend(
                [
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/opentype/noto/NotoSansCJKSC-Regular.otf",
                    "/usr/share/fonts/opentype/noto/NotoSansCJK.ttc",
                    "/usr/share/fonts/opentype/source-han-sans/SourceHanSansCN-Regular.otf",
                    "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
                ]
                .into_iter()
                .map(PathBuf::from),
            );
        }
        paths
    }

    let mut fonts = egui::FontDefinitions::default();

    let mut installed = false;
    for path in candidates() {
        if let Ok(bytes) = fs::read(&path) {
            let key = format!("CJKFallback({})", path.display());
            fonts
                .font_data
                .insert(key.clone(), Arc::new(egui::FontData::from_owned(bytes)));
            // Add as fallback for both families (monospace first to satisfy .monospace() usage)
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push(key.clone());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push(key.clone());
            installed = true;
            break;
        }
    }

    if !installed {
        log::warn!(
            "No system CJK font found. Set UDPTCP_CJK_FONT=/path/to/font.(ttc|ttf|otf) to override."
        );
    }

    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size((1114.0, 588.0))
            .with_title("UDP TCP"),
        ..Default::default()
    };
    eframe::run_native(
        "default application title",
        native_options,
        Box::new(|cc| {
            // Install system CJK fallback (YaHei on Windows, PingFang on macOS, etc.)
            install_cjk_fallback(&cc.egui_ctx);

            Ok(Box::new(App::new()))
        }),
    )
}
