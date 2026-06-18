export type DocumentKind = "markdown" | "email" | "image" | "audio" | "video" | "text" | "docx" | "unsupported";

export type DocumentPayload = {
  path: string | null;
  fileName: string;
  kind: DocumentKind;
  title: string;
  source: string;
  markdown?: string;
  email?: EmailPayload;
  image?: ImagePayload;
  media?: MediaPayload;
  text?: TextPayload;
  info: FileInfoPayload;
  error?: string;
};

export type EmailPayload = {
  headers: EmailHeader[];
  from?: string;
  to?: string;
  subject?: string;
  date?: string;
  bodyHtml?: string;
  bodyText?: string;
};

export type EmailHeader = {
  name: string;
  value: string;
};

export type ImagePayload = {
  dataUrl: string;
  mimeType: string;
  renderNote?: string;
};

export type MediaPayload = {
  dataUrl: string;
  mimeType: string;
  renderNote?: string;
};

export type TextPayload = {
  text: string;
  language?: string;
};

export type FileInfoPayload = {
  general: InfoEntry[];
  sections: InfoSection[];
};

export type InfoSection = {
  title: string;
  entries: InfoEntry[];
};

export type InfoEntry = {
  label: string;
  value: string;
};

export type ViewerDefinition = {
  kind: DocumentKind;
  label: string;
  canShowSource: boolean;
};
