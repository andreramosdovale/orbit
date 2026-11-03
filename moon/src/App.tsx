import { useEffect, useState } from "react";
import { commands } from "./bindings";
import "./App.css";

function App() {
  const [version, setVersion] = useState("");

  useEffect(() => {
    commands.appVersion().then(setVersion);
  }, []);

  return (
    <main className="container">
      <h1>Orbit</h1>
      <p className="version">v{version}</p>
    </main>
  );
}

export default App;
