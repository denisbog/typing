use crate::{
    application_types::{Article, Data},
    error_template::{AppError, ErrorTemplate},
    translation::{get_translations, TranslationRequest},
    translation_page::{ArticlePage, TranslationPage},
};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let (translation_input, set_translation_input) = create_signal("".to_string());

    let data = load_data();
    let (translation_post, set_translation_post) = create_signal(data);
    let (input_popup, set_input_popup) = create_signal(false);

    let input_popup_component = move || {
        if input_popup() {
            view! {
                <div class="fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity">
                    <div class="fixed inset-0 z-10 w-screen overflow-y-auto">
                        <div class="flex min-h-full items-end justify-center p-4 text-center sm:items-center sm:p-0 lg:p-5">
                            <div class="flex relative transform overflow-hidden bg-gray-100 shadow-xl transition-all w-full h-full">
                                <div class="flex flex-1 flex-col bg-white px-4 pb-4 pt-5 sm:p-6 sm:pb-4">
                                    <textarea
                                        class="h-80"
                                        type="textarea"
                                        placeholder="type here your text"
                                        prop:value=translation_input
                                        on:input=move |event| {
                                            set_translation_input.set(event_target_value(&event));
                                        }
                                    >
                                    </textarea>

                                    <div class="p-2">
                                        <input
                                            class="p-2 m-1 shadow-md rounded bg-green-100"
                                            type="button"
                                            value="Translate"
                                            on:click=move |_event| {
                                                let temp = translation_input.get();
                                                logging::log!("passing argument: {}", temp);
                                                set_input_popup.set(false);
                                                spawn_local(async move {
                                                    let request = TranslationRequest::from_str(&temp);
                                                    let response = get_translations(request.clone()).await.unwrap();
                                                    set_translation_post
                                                        .update(|data| {
                                                            data.articles
                                                                .push(Article::from_pair(request.src, response.translated));
                                                        });
                                                });
                                            }
                                        />

                                        <input
                                            class="p-2 m-1 shadow-md rounded bg-gray-100 text-gray-400"
                                            type="button"
                                            value="Close"
                                            on:click=move |_event| {
                                                set_input_popup.set(false);
                                            }
                                        />

                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            }.into_view()
        } else {
            view! {}.into_view()
        }
    };

    view! {
        <Html class="snap-y snap-y-mandatory"/>

        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/typing.css"/>

        // sets the document title
        <Title text="Typing app"/>

        <Body class="h-screen bg-gray-400 text-5xl lg:text-3xl text-gray-900"/>
        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors/> }.into_view()
        }>
            <div class="p-3 pt-7 lg:text-3xl text-5xl font-bold text-gray-100 font-mono w-screen items-center flex flex-col snap-start">
                <a href="/"><div>Learn German by typing!</div></a>
            </div>
            <div class="p-3 pt-7 lg:text-3xl text-5xl font-bold text-gray-100 font-mono w-screen items-center flex flex-col snap-start">
                <div on:click=move |_event| set_input_popup(true)>Update text!</div>
            </div>
            <main class="w-screen flex flex-col items-center">
                <Routes>
                    <Route
                        path=""
                        view=move || view! { <TranslationPage data=translation_post/> }
                />
                    <Route
                        path="/article/:id"
                        view=move || view! { <ArticlePage data=translation_post.get()/> }
                    />
                </Routes>
                <div>{move || input_popup_component}</div>
            </main>
        </Router>
    }
}

fn load_data() -> Data {
    let sentances = vec![
        "Mit intelligenten Stromzählern können Verbraucher selbst am Energiemarkt teilnehmen. Wie Sie Geld sparen und sogar welches verdienen.",
        "Die Preise an der Strombörse fahren an vielen Tagen des Jahres Achterbahn: Sie vervielfachen sich oft binnen weniger Stunden, um kurz darauf genauso rasant wieder abzustürzen. Mitunter gar in den negativen Bereich – die Versorger bekommen dann Geld dafür, dass sie Strom abnehmen.",
        "Für die Verbraucher hat dieses Auf und Ab keine unmittelbaren Folgen, da sie für ihren Strom in der Regel stets den gleichen Preis zahlen. Damit gewinnen sie Sicherheit. Das bedeutet aber auch, dass sie nichts davon haben, wenn es an der Börse mal wieder abwärtsgeht.",
        "Mit dem schrittweisen Einzug intelligenter Stromzähler in die Haushalte ändert sich das nun aber: Sie geben Bürgern die Möglichkeit, am Strommarkt teilzuhaben und damit Geld zu sparen – und sogar zu verdienen."
];

    let translations = vec![
		"With intelligent power meters, consumers can participate in the energy market themselves. How to save money and even earn what.",
		"The prices at the power exchange run roller coaster on many days of the year: they often multiply within a few hours to crash just as rapidly shortly afterwards. Sometimes even into the negative area – the suppliers then get money for them to lose electricity.",
		"For consumers, this ups and downs have no immediate consequences, as they usually pay the same price for their electricity. This means that they gain security. This also means that they have nothing of it if it goes down again on the stock market.",
		"But with the gradual introduction of smart electricity meters into households, this is now changing: they give citizens the opportunity to participate in the electricity market and thus save – and even earn – money."
    ];
    let data = Data {
        articles: vec![
            Article::from_pair(
                sentances.iter().map(|item| item.to_string()).collect(),
                translations.iter().map(|item| item.to_string()).collect(),
            ),
            Article::from_pair(
                sentances.iter().map(|item| item.to_string()).collect(),
                translations.iter().map(|item| item.to_string()).collect(),
            ),
        ],
    };
    data
}
