use leptos::*;
use typing::components::Sentance;
fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}

#[component]
fn App() -> impl IntoView {
    let sentances = vec![
        "Mit intelligenten Stromzählern können Verbraucher selbst am Energiemarkt teilnehmen. Wie Sie Geld sparen und sogar welches verdienen.",
        "Die Preise an der Strombörse fahren an vielen Tagen des Jahres Achterbahn: Sie vervielfachen sich oft binnen weniger Stunden, um kurz darauf genauso rasant wieder abzustürzen. Mitunter gar in den negativen Bereich – die Versorger bekommen dann Geld dafür, dass sie Strom abnehmen.",
        "Für die Verbraucher hat dieses Auf und Ab keine unmittelbaren Folgen, da sie für ihren Strom in der Regel stets den gleichen Preis zahlen. Damit gewinnen sie Sicherheit. Das bedeutet aber auch, dass sie nichts davon haben, wenn es an der Börse mal wieder abwärtsgeht.",
        "Mit dem schrittweisen Einzug intelligenter Stromzähler in die Haushalte ändert sich das nun aber: Sie geben Bürgern die Möglichkeit, am Strommarkt teilzuhaben und damit Geld zu sparen – und sogar zu verdienen."
];
    sentances
        .iter()
        .map(|item| view! { <Sentance text=item/> })
        .collect_view()
}
