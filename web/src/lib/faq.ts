export const questions = [
  {
    question: "Which installer do I choose?",
    answer:
      "Open the Apple menu and choose About This Mac. If it says Chip, download Apple Silicon. If it says Processor, download Intel.",
  },
  {
    question: "How do I install it?",
    answer:
      "Open the disk image and drag LiveUse to Applications. When a build is not signed, right click LiveUse the first time and choose Open.",
  },
  {
    question: "What does macOS ask for?",
    answer:
      "Screen recording, for the captures an agent needs. Incoming connections, so the API can listen on your local network.",
  },
  {
    question: "Which board does it need?",
    answer:
      "Today, a Seeed XIAO SAMD21 with the sketch running. The bootloader does not start the service. Other boards are being explored, and a smaller board that plugs straight into the computer is in its first steps.",
  },
  {
    question: "Who can move the cursor?",
    answer:
      "Anyone with the token on that local network. The Share tab can issue a new token. The previous one stops working.",
  },
  {
    question: "How do I quit?",
    answer:
      "Closing the window leaves LiveUse in the menu bar. Quit in that menu ends the app. A click on the Dock icon opens the window again.",
  },
  {
    question: "How do I stop an action?",
    answer:
      "Press Control, Option, Command, and Escape together. That aborts the current operation and opens the window. Stop in the chat does the same thing.",
  },
  {
    question: "Is there a Linux or Windows app?",
    answer:
      "The installer on this page is macOS. Desktop builds for Linux and Windows are in progress.",
  },
  {
    question: "Where are the release notes?",
    answer:
      "On the public GitHub release for the version shown on this page. The link sits with the two installers.",
  },
] as const;
