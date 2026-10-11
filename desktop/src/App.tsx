import { Toaster } from "@/components/ui/sonner";
import { MainWindow } from "@/main-window";
import { Overlay } from "@/overlay";

function App() {
  const overlay = new URLSearchParams(window.location.search).get("view") === "overlay";
  if (overlay) {
    return <Overlay />;
  }
  return (
    <>
      <MainWindow />
      <Toaster />
    </>
  );
}

export default App;
