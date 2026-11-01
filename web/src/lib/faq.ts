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
    question: "What is LuLu?",
    answer:
      "LuLu is the USB device LiveUse drives. It is a printed case with a Seeed XIAO SAMD21 inside, running the LiveUse firmware. The bootloader does not start the service. Other boards are being explored, and a smaller device that plugs straight into the computer is in its first steps.",
  },
  {
    question: "Do I flash LuLu myself?",
    answer:
      "No. The Device tab checks the published firmware and writes it, or updates it. The image and its checksum are also public, with bossac instructions, in the liveuse-releases repository.",
  },
  {
    question: "How do I build LuLu?",
    answer:
      "Print the two-part case in PLA or PETG, without supports, and press the XIAO SAMD21 into it. LuLu measures 25.0 by 21.6 by 8.9 mm, and the USB-C stays open. The 3MF and STL files are in the xiao-samd21-case repository.",
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
