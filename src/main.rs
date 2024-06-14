use leptos::*;
use typing::components::Sentance;
fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}

#[component]
fn App() -> impl IntoView {
    let sentance ="Dass überhaupt noch Euro und Dollar in Moskau landen und gegen Rubel getauscht werden können, ist dabei wenig überraschend. Russland erwirtschaftet Jahr für Jahr hohe Überschüsse durch den Verkauf von Öl, Gas und anderen Rohstoffen. Viele davon werden auch nach Kriegsbeginn noch in westlichen Währungen bezahlt, auch wenn der Anteil asiatischer Währungen wie des chinesischen Yuan steigt. Russlands Exporteure also haben Dollar und Euro, während andere Firmen im Ausland dringend benötigte Einfuhren kaufen müssen. Auch von diesen müssen viele immer noch in Euro und Dollar bezahlt werden. Grob vereinfacht gesagt: Russlands Rohstoffexporteure verkaufen in Moskau ihre Dollars, die Importeure wiederum kaufen sie dort. Einer der zentralen Handelspunkte für solche Geschäfte war bislang: die Moskauer Börse.";
    view! {
        <Sentance text=sentance/>
        <Sentance text=sentance/>
    }
}
