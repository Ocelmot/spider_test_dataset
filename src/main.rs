use std::{io, path::PathBuf};

use spider_client::{
    message::{
        DatasetMessage, DatasetPath, Message, UiElement, UiElementContent, UiElementContentPart,
        UiElementKind, UiInput, UiMessage, UiPageManager, UiPath,
    },
    ClientChannel, ClientResponse, SpiderClientBuilder,
};

#[tokio::main]
async fn main() {
    let client_path = PathBuf::from("client_state.dat");

    let mut builder = SpiderClientBuilder::load_or_set(&client_path, |builder| {
        builder.enable_fixed_addrs(true);
        builder.set_fixed_addrs(vec!["localhost:1930".into()]);
    });

    builder.try_use_keyfile("spider_keyfile.json").await;

    let mut client_channel = builder.start(true);
    let mut state = State::init(&mut client_channel).await;

    loop {
        match client_channel.recv().await {
            Some(ClientResponse::Message(msg)) => {
                state.msg_handler(&mut client_channel, msg).await;
            }
            Some(ClientResponse::Denied(_)) => break,
            None => break, //  done!
            _ => {}
        }
    }
}

struct State {}

impl State {
    async fn init(client: &mut ClientChannel) -> Self {
        // Subscribe to dataset
        let dataset_path = DatasetPath::new_private(vec![String::from("Test")]);
        let msg = Message::Dataset(DatasetMessage::Subscribe {
            path: dataset_path.clone(),
        });

        client.send(msg).await;

        // setup page
        let id = client.id();
        let mut test_page = UiPageManager::new(id.clone(), "Dataset Test Page");
        let mut root = test_page
            .get_element_mut(&UiPath::root())
            .expect("all pages have a root");
        root.set_kind(UiElementKind::Rows);
        root.append_child(UiElement::from_string("Data:"));
        root.append_child({
            let mut element = UiElement::new(UiElementKind::Rows);
            element.set_dataset(Some(dataset_path.clone().resolve(id.clone())));
            element.append_child({
                let mut child = UiElement::new(UiElementKind::Columns);
                child.append_child({
                    let mut child = UiElement::new(UiElementKind::Text);
                    let mut content = UiElementContent::new();
                    content.add_part(UiElementContentPart::Data(vec![]));
                    child.set_content(content);
                    child
                });
                child.append_child(UiElement::new(UiElementKind::Spacer));
                child.append_child({
                    let mut child = UiElement::new(UiElementKind::Button);
                    child.set_id("delete row");
                    child.set_selectable(true);
                    child.set_text("Delete!");
                    child
                });
                child
            });
            element
        });

        root.append_child({
            let mut element = UiElement::from_string("Add element");
            element.set_kind(UiElementKind::Button);
            element.set_selectable(true);
            element.set_id("add_button");
            element
        });

        root.append_child({
            let mut element = UiElement::from_string("Remove element");
            element.set_kind(UiElementKind::Button);
            element.set_selectable(true);
            element.set_id("remove_button");
            element
        });
        root.append_child({
            let mut element = UiElement::from_string("Text Input");
            element.set_kind(UiElementKind::TextEntry);
            element.set_selectable(true);
            element.set_id("TextInput");
            element
        });

        drop(root);

        test_page.get_changes(); // clear changes to synch, since we are going to send the whole page at first. This
                                 // Could instead set the initial elements with raw and then recalculate ids
        let msg = Message::Ui(UiMessage::SetPage(test_page.get_page().clone()));
        client.send(msg).await;

        // Create self
        Self {}
    }

    async fn msg_handler(&mut self, client: &mut ClientChannel, msg: Message) {
        match msg {
            Message::Ui(msg) => self.ui_handler(client, msg).await,
            Message::Dataset(msg) => self.dataset_handler(client, msg).await,
            Message::Router(_) => {}
            Message::Error(_) => {}
        }
    }

    async fn dataset_handler(&mut self, _client: &mut ClientChannel, msg: DatasetMessage) {
        println!("Message: {:?}", msg);
        if let DatasetMessage::Dataset { path: _, data } = msg {
            println!("Data: {:?}", data.get(0))
        }
    }

    async fn ui_handler(&mut self, client: &mut ClientChannel, msg: UiMessage) {
        match msg {
            UiMessage::Subscribe => {}
            UiMessage::Pages(_) => {}
            UiMessage::GetPage(_) => {}
            UiMessage::Page(_) => {}
            UiMessage::UpdateElementsFor(_, _) => {}
            UiMessage::InputFor(_, _, _, _) => {}
            UiMessage::SetPage(_) => {}
            UiMessage::ClearPage => {}
            UiMessage::UpdateElements(_) => {}
            UiMessage::Input(element_id, dataset_ids, change) => {
                let dataset_path = DatasetPath::new_private(vec![String::from("Test")]);
                match element_id.as_str() {
                    "add_button" => {
                        let data = spider_client::message::DatasetData::String(String::from(
                            "added data!",
                        ));
                        let msg = Message::Dataset(DatasetMessage::Append {
                            path: dataset_path,
                            data: data,
                        });
                        client.send(msg).await;
                    }
                    "remove_button" => {
                        let msg = Message::Dataset(DatasetMessage::DeleteElement {
                            path: dataset_path,
                            id: 0,
                        });
                        client.send(msg).await;
                    }
                    "delete row" => {
                        let msg = Message::Dataset(DatasetMessage::DeleteElement {
                            path: dataset_path,
                            id: *dataset_ids.last().unwrap_or(&0),
                        });
                        client.send(msg).await;
                    }
                    "TextInput" => {
                        if let UiInput::Text(text) = change {
                            let data = spider_client::message::DatasetData::String(text);
                            let msg = Message::Dataset(DatasetMessage::Append {
                                path: dataset_path,
                                data: data,
                            });
                            client.send(msg).await;
                        }
                    }
                    _ => return,
                }
            }
            UiMessage::Dataset(_, _) => {}
        }
    }
}
