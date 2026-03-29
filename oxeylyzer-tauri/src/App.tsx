import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

function App() {
    const [greetMsg, setGreetMsg] = createSignal("");
    const [name, setName] = createSignal("");

    async function greet() {
        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        let what = await invoke("what");
        let greet = await invoke("greet", { name: name() });
        setGreetMsg(`${greet}! ${what}`);
    }

    return (
        <main class="bg-gray-800 w-screen h-screen absolute">
            <h1>Welcome to Tauri + Solid</h1>
            <h2 class="text-5xl">dofsmeye</h2>

            <form
                class="row"
                onSubmit={(e) => {
                    e.preventDefault();
                    greet();
                }}
            >
                <input
                    id="greet-input"
                    onChange={(e) => setName(e.currentTarget.value)}
                    placeholder="Enter a name..."
                />
                <button type="submit">Greet</button>
            </form>
            <p>{greetMsg()}</p>
        </main>
    );
}

export default App;
