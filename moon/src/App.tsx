import { useEffect, useState } from "react";
import { commands } from "./bindings";
import { Register } from "./features/auth/Register";
import "./App.css";

function App() {
  const [version, setVersion] = useState("");

  useEffect(() => {
    commands.appVersion().then(setVersion);
  }, []);

  return (
    <main className="container">
      <Register />
      <p className="version">Orbit v{version}</p>
    </main>
  );
}

export default App;
