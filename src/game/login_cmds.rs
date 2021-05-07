use legion::*;
use crate::game::objects::MudSession;
use crate::net::{ProtocolComponent, ProtocolOutEvent, ProtocolEvent};
use crate::game::resources::{PendingUserCreations, PendingUserLogins};
use crate::mudstring::text::{Text};


pub struct LoginCommands {
    pub cmds: Vec<LoginCmd>
}

impl Default for LoginCommands {
    fn default() -> Self {
        let mut cmds = Vec::new();

        cmds.push(LoginCmd{name: "connect".to_string(), aliases: Default::default(),
            func: login_login_command, help: "does a login".to_string(),
            syntax: "connect <username>=<password>".to_string(),
            shorthelp: "connect <username>=<password>".to_string()});

        cmds.push(LoginCmd{name: "create".to_string(), aliases: Default::default(),
            func: login_create_command, help: "creates an account".to_string(),
            syntax: "create <username>=<password>".to_string(),
            shorthelp: "create <username>=<password>".to_string()});

        cmds.push(LoginCmd{name: "help".to_string(), aliases: Default::default(),
            func: login_help_command, help: "displays help".to_string(),
            syntax: "help [<topic>]".to_string(),
            shorthelp: "help [<topic>]".to_string()});


        Self {
            cmds
        }
    }
}

impl LoginCommands {
    pub fn execute(&mut self, prot: &mut ProtocolComponent, command: String) {
        let split: Vec<&str> = command.splitn(2, ' ').collect();
        let comm = split[0].trim();
        let args = if split.len() == 2 {
            split[1].trim()
        } else {
            ""
        };
        for cmd in self.cmds.iter() {
            if cmd.name_match(comm) {

                (cmd.func)(prot, args.to_string(), &self.cmds);
                return;
            }
        }
        prot.out_buffer.push_back(ProtocolOutEvent::Line(Text::from(format!("Sorry, {} that isn't a command. Type 'help' for help.", command).as_ref())));
    }
}


pub struct LoginCmd {
    pub name: String,
    pub aliases: Vec<String>,
    pub func: fn(&mut ProtocolComponent, command: String, &Vec<LoginCmd>),
    pub help: String,
    pub syntax: String,
    pub shorthelp: String,
}

impl LoginCmd {
    pub fn name_match(&self, command: impl AsRef<str>) -> bool {
        let upper = command.as_ref().to_uppercase();

        if self.name.to_uppercase() == upper {
            true
        } else {
            for ali in self.aliases.iter() {
                if ali.to_uppercase() == upper {
                    return true
                }
            }
            return false
        }
    }

    pub fn args(command: impl AsRef<str>) -> String {
        let ref_cmd = command.as_ref();
        let split: Vec<&str> = ref_cmd.splitn(2, ' ').collect();
        if split.len() == 2 {
            split[1].to_string()
        } else {
            "".to_string()
        }
    }
}

pub fn login_create_command(prot: &mut ProtocolComponent, args: String, cmds: &Vec<LoginCmd>) {
    let args: Vec<&str> = args.splitn(2, '=').collect();
    if args.len() == 2 {
        let username = args[0].trim();
        let password = args[1].trim();
        if (password.len() == 0) | (username.len() == 0) {
            prot.out_buffer.push_back(ProtocolOutEvent::Line(Text::from("SYNTAX: create <username>=<password>")));
        } else {
            prot.in_buffer.push_back(ProtocolEvent::CreateUser(username.to_string(), password.to_string()));
        }
    } else {
        prot.out_buffer.push_back(ProtocolOutEvent::Line(Text::from("SYNTAX: create <username>=<password>")));
    }
}

pub fn login_login_command(prot: &mut ProtocolComponent, args: String, cmds: &Vec<LoginCmd>) {
    let args: Vec<&str> = args.splitn(2, '=').collect();
    if args.len() == 2 {
        let username = args[0].trim();
        let password = args[1].trim();
        if (password.len() == 0) | (username.len() == 0) {
            prot.out_buffer.push_back(ProtocolOutEvent::Line(Text::from("SYNTAX: connect <username>=<password>")));
        } else {
            prot.in_buffer.push_back(ProtocolEvent::Login(username.to_string(), password.to_string()));
        }
    } else {
        prot.out_buffer.push_back(ProtocolOutEvent::Line(Text::from("SYNTAX: connect <username>=<password>")));
    }
}

pub fn login_help_command(prot: &mut ProtocolComponent, args: String, cmds: &Vec<LoginCmd>) {

    if args.is_empty() {
        let mut out = String::new();
        for cmd in cmds {
            out += format!("{} | {} | {}\n", cmd.name, cmd.syntax, cmd.shorthelp).as_str();
        }
        prot.out_buffer.push_back(ProtocolOutEvent::Line(Text::from(out.as_ref())));
        return
    } else {
        let results: Vec<&LoginCmd> = cmds.iter().filter(|x| x.name_match(&args)).collect();
        if let Some(res) = results.first() {
            prot.out_buffer.push_back(ProtocolOutEvent::Line(Text::from(res.help.as_ref())));
        } else {
            prot.out_buffer.push_back(ProtocolOutEvent::Line(Text::from("Sorry, no help found for that! try help without arguments")));
        }
        return
    }
}