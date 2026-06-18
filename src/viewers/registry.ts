import type { DocumentKind, ViewerDefinition } from "./types";

export const viewerRegistry: Record<DocumentKind, ViewerDefinition> = {
  markdown: {
    kind: "markdown",
    label: "Markdown",
    canShowSource: true,
  },
  email: {
    kind: "email",
    label: "Email",
    canShowSource: true,
  },
  image: {
    kind: "image",
    label: "Image",
    canShowSource: false,
  },
  audio: {
    kind: "audio",
    label: "Audio",
    canShowSource: false,
  },
  video: {
    kind: "video",
    label: "Video",
    canShowSource: false,
  },
  text: {
    kind: "text",
    label: "Text",
    canShowSource: true,
  },
  docx: {
    kind: "docx",
    label: "Word Document",
    canShowSource: false,
  },
  unsupported: {
    kind: "unsupported",
    label: "Unsupported",
    canShowSource: false,
  },
};
