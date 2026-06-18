import { invoke } from "@tauri-apps/api/core";
import DOMPurify from "dompurify";
import { marked } from "marked";
import { useEffect, useMemo, useState } from "preact/hooks";
import { viewerRegistry } from "../viewers/registry";
import type { DocumentPayload, InfoEntry } from "../viewers/types";

type ViewMode = "preview" | "source";
type ZoomMode = "fit" | "actual" | "custom";

const welcomeDocument: DocumentPayload = {
  path: null,
  fileName: "Rads Quick Viewer",
  kind: "unsupported",
  title: "Open a file",
  source: "",
  info: {
    general: [],
    sections: [],
  },
};

marked.use({
  gfm: true,
  breaks: false,
});

export function App() {
  const [document, setDocument] = useState<DocumentPayload>(welcomeDocument);
  const [viewMode, setViewMode] = useState<ViewMode>("preview");
  const [zoomMode, setZoomMode] = useState<ZoomMode>("fit");
  const [customZoom, setCustomZoom] = useState(100);
  const [showAssociationNotice, setShowAssociationNotice] = useState(() => {
    return window.localStorage.getItem("rads-quick-viewer-hide-association-notice") !== "true";
  });
  const [showInfo, setShowInfo] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [status, setStatus] = useState("Ready");

  useEffect(() => {
    void loadInitialFile();
  }, []);

  const viewer = viewerRegistry[document.kind];
  const canShowSource = viewer.canShowSource && document.source.length > 0;
  const showViewModeControls = canShowSource;
  const isZoomableVisual =
    viewMode === "preview" &&
    ((document.kind === "image" && !!document.image?.dataUrl) ||
      (document.kind === "video" && !!document.media?.dataUrl));

  const markdownHtml = useMemo(() => {
    if (document.kind !== "markdown") {
      return "";
    }

    return DOMPurify.sanitize(marked.parse(document.markdown ?? document.source, { async: false }) as string);
  }, [document]);

  const emailHtml = useMemo(() => {
    if (document.kind !== "email" || !document.email?.bodyHtml) {
      return "";
    }

    return DOMPurify.sanitize(document.email.bodyHtml, {
      ADD_ATTR: ["target", "rel"],
    });
  }, [document]);

  async function loadInitialFile() {
    try {
      const initialPath = await invoke<string | null>("get_initial_file");
      if (initialPath) {
        await loadFile(initialPath);
      }
    } catch (error) {
      setStatus(errorToMessage(error));
    }
  }

  async function loadFile(path: string) {
    setIsLoading(true);
    setStatus("Opening...");
    try {
      const payload = await invoke<DocumentPayload>("read_document", { path });
      setDocument(payload);
      setViewMode("preview");
      setShowInfo(false);
      if (payload.kind === "image" || payload.kind === "video") {
        setZoomMode("fit");
        setCustomZoom(100);
      }
      setStatus(payload.error ?? "Ready");
    } catch (error) {
      setStatus(errorToMessage(error));
    } finally {
      setIsLoading(false);
    }
  }

  async function chooseFile() {
    try {
      const path = await invoke<string | null>("choose_file");
      if (path) {
        await loadFile(path);
      }
    } catch (error) {
      setStatus(errorToMessage(error));
    }
  }

  async function openSettings() {
    try {
      await invoke("open_default_app_settings");
      setStatus("Opened system default-app settings");
    } catch (error) {
      setStatus(errorToMessage(error));
    }
  }

  async function navigateSibling(direction: -1 | 1) {
    if (!document.path || isLoading) {
      return;
    }

    try {
      const nextPath = await invoke<string | null>("get_sibling_file", {
        path: document.path,
        direction,
      });
      if (nextPath) {
        await loadFile(nextPath);
      } else {
        setStatus(direction < 0 ? "No previous file" : "No next file");
      }
    } catch (error) {
      setStatus(errorToMessage(error));
    }
  }

  function dismissAssociationNotice() {
    window.localStorage.setItem("rads-quick-viewer-hide-association-notice", "true");
    setShowAssociationNotice(false);
  }

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target;
      if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) {
        return;
      }

      if (event.key === "Escape" && showInfo) {
        event.preventDefault();
        setShowInfo(false);
        return;
      }

      if (showInfo) {
        return;
      }

      if (event.key === "ArrowLeft") {
        event.preventDefault();
        void navigateSibling(-1);
      } else if (event.key === "ArrowRight") {
        event.preventDefault();
        void navigateSibling(1);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [document.path, isLoading, showInfo]);

  return (
    <main class="shell">
      <header class="toolbar">
        <div class="identity">
          <img class="app-mark" src="/icon.png" alt="" aria-hidden="true" />
          <div class="title-block">
            <div class="file-line">
              <button
                type="button"
                class="nav-button"
                title="Previous file"
                disabled={!document.path || isLoading}
                onClick={() => void navigateSibling(-1)}
              >
                ‹
              </button>
              <span class="file-title" title={document.path ?? undefined}>
                {document.fileName}
              </span>
              <button
                type="button"
                class="nav-button"
                title="Next file"
                disabled={!document.path || isLoading}
                onClick={() => void navigateSibling(1)}
              >
                ›
              </button>
            </div>
          </div>
        </div>

        <div class="toolbar-actions">
          {showViewModeControls ? (
            <div class="segmented" aria-label="View mode">
              <button
                type="button"
                class={viewMode === "preview" ? "active" : ""}
                onClick={() => setViewMode("preview")}
              >
                Preview
              </button>
              <button
                type="button"
                class={viewMode === "source" ? "active" : ""}
                onClick={() => setViewMode("source")}
              >
                Source
              </button>
            </div>
          ) : null}

          {isZoomableVisual ? (
            <VisualZoomControls
              zoomMode={zoomMode}
              customZoom={customZoom}
              onZoomModeChange={(mode) => {
                setZoomMode(mode);
                setCustomZoom(100);
              }}
              onCustomZoomChange={(value) => {
                setCustomZoom(value);
                setZoomMode("custom");
              }}
            />
          ) : null}

          {document.path && !document.error ? (
            <button type="button" class="ghost-button" onClick={() => setShowInfo(true)}>
              Info
            </button>
          ) : null}
          <button type="button" class="primary-button" disabled={isLoading} onClick={chooseFile}>
            Open
          </button>
        </div>
      </header>

      {showAssociationNotice ? (
        <section class="notice-bar">
          <span>Do you want to associate Rads Quick Viewer with supported file types?</span>
          <div>
            <button type="button" class="ghost-button" onClick={openSettings}>
              Open Windows Settings
            </button>
            <button type="button" class="icon-button" aria-label="Dismiss" onClick={dismissAssociationNotice}>
              ×
            </button>
          </div>
        </section>
      ) : null}

      <section class={`content ${isZoomableVisual ? "visual-content" : ""}`}>
        {document.error ? <ErrorState message={document.error} /> : null}
        {!document.error && viewMode === "source" ? <SourceView source={document.source} /> : null}
        {!document.error && viewMode === "preview" ? (
          <PreviewView
            document={document}
            markdownHtml={markdownHtml}
            emailHtml={emailHtml}
            zoomMode={zoomMode}
            customZoom={customZoom}
            onWheelZoom={(delta) => updateCustomZoom(delta)}
          />
        ) : null}
      </section>

      {showInfo ? <InfoDialog document={document} onClose={() => setShowInfo(false)} /> : null}
    </main>
  );

  function updateCustomZoom(delta: number) {
    setZoomMode("custom");
    setCustomZoom((current) => clamp(current + delta, 10, 400));
  }
}

function PreviewView({
  document,
  markdownHtml,
  emailHtml,
  zoomMode,
  customZoom,
  onWheelZoom,
}: {
  document: DocumentPayload;
  markdownHtml: string;
  emailHtml: string;
  zoomMode: ZoomMode;
  customZoom: number;
  onWheelZoom: (delta: number) => void;
}) {
  if (document.kind === "markdown") {
    return <article class="document markdown-body" dangerouslySetInnerHTML={{ __html: markdownHtml }} />;
  }

  if (document.kind === "email" && document.email) {
    const email = document.email;
    return (
      <article class="document email-view">
        <header class="email-header">
          <h1>{email.subject || document.title}</h1>
          <dl>
            {email.from ? <HeaderRow label="From" value={email.from} /> : null}
            {email.to ? <HeaderRow label="To" value={email.to} /> : null}
            {email.date ? <HeaderRow label="Date" value={email.date} /> : null}
          </dl>
        </header>
        {emailHtml ? (
          <section class="email-html" dangerouslySetInnerHTML={{ __html: emailHtml }} />
        ) : (
          <pre class="plain-text">{email.bodyText ?? ""}</pre>
        )}
      </article>
    );
  }

  if (document.kind === "image" && document.image) {
    return (
      <ImageView
        document={document}
        zoomMode={zoomMode}
        customZoom={customZoom}
        onWheelZoom={onWheelZoom}
      />
    );
  }

  if ((document.kind === "audio" || document.kind === "video") && document.media) {
    return (
      <MediaView
        document={document}
        zoomMode={zoomMode}
        customZoom={customZoom}
        onWheelZoom={onWheelZoom}
      />
    );
  }

  if ((document.kind === "text" || document.kind === "docx") && document.text) {
    return (
      <article class="document text-view">
        <header class="text-header">
          <h1>{document.title}</h1>
          <span>{document.text.language ?? viewerRegistry[document.kind].label}</span>
        </header>
        <pre>{document.text.text}</pre>
      </article>
    );
  }

  return (
    <section class="empty-state">
      <h1>{document.title}</h1>
      <p>Rads Quick Viewer opens Markdown, email, images, audio, video, text, and lightweight .docx files.</p>
    </section>
  );
}

function VisualZoomControls({
  zoomMode,
  customZoom,
  onZoomModeChange,
  onCustomZoomChange,
}: {
  zoomMode: ZoomMode;
  customZoom: number;
  onZoomModeChange: (mode: ZoomMode) => void;
  onCustomZoomChange: (value: number) => void;
}) {
  return (
    <div class="visual-controls" aria-label="Zoom controls">
      <div class="segmented zoom-segmented">
        <button
          type="button"
          class={zoomMode === "fit" ? "active" : ""}
          onClick={() => onZoomModeChange("fit")}
        >
          Fit
        </button>
        <button
          type="button"
          class={zoomMode === "actual" ? "active" : ""}
          onClick={() => onZoomModeChange("actual")}
        >
          1:1
        </button>
      </div>
      <label class="zoom-slider">
        <span>{(customZoom / 100).toFixed(2)}x</span>
        <input
          type="range"
          min="10"
          max="400"
          step="5"
          value={customZoom}
          onInput={(event) => onCustomZoomChange(Number(event.currentTarget.value))}
        />
      </label>
    </div>
  );
}

function MediaView({
  document,
  zoomMode,
  customZoom,
  onWheelZoom,
}: {
  document: DocumentPayload;
  zoomMode: ZoomMode;
  customZoom: number;
  onWheelZoom: (delta: number) => void;
}) {
  const media = document.media!;
  const isVideo = document.kind === "video";
  const className = `media-stage ${isVideo ? "visual-stage" : ""} ${zoomMode === "fit" ? "fit" : ""}`;
  const style =
    zoomMode === "custom"
      ? { width: `${customZoom}%` }
      : zoomMode === "actual"
        ? { width: "auto" }
        : undefined;

  return (
    <article class={`document media-view ${isVideo ? "video-view" : "audio-view"}`}>
      <header class="media-header">
        <h1>{document.title}</h1>
        <span>{media.mimeType}</span>
      </header>
      {media.renderNote ? <div class="media-note">{media.renderNote}</div> : null}
      <div class={className} onWheel={isVideo ? (event) => handleWheelZoom(event, onWheelZoom) : undefined}>
        {isVideo ? (
          <video controls src={media.dataUrl} style={style} />
        ) : (
          <audio controls src={media.dataUrl} />
        )}
      </div>
    </article>
  );
}

function ImageView({
  document,
  zoomMode,
  customZoom,
  onWheelZoom,
}: {
  document: DocumentPayload;
  zoomMode: ZoomMode;
  customZoom: number;
  onWheelZoom: (delta: number) => void;
}) {
  const image = document.image!;
  const className = `image-stage visual-stage ${zoomMode === "fit" ? "fit" : ""}`;
  const style =
    zoomMode === "custom"
      ? { width: `${customZoom}%` }
      : zoomMode === "actual"
        ? { width: "auto" }
        : undefined;

  return (
    <article class="image-view">
      {image.renderNote ? <div class="image-note">{image.renderNote}</div> : null}
      {image.dataUrl ? (
        <div class={className} onWheel={(event) => handleWheelZoom(event, onWheelZoom)}>
          <img src={image.dataUrl} alt={document.title} style={style} />
        </div>
      ) : (
        <section class="empty-state">
          <h1>Image preview unavailable</h1>
          <p>{image.renderNote ?? "This image format could not be rendered."}</p>
        </section>
      )}
    </article>
  );
}

function handleWheelZoom(event: WheelEvent, onWheelZoom: (delta: number) => void) {
  if (!event.ctrlKey) {
    return;
  }

  event.preventDefault();
  onWheelZoom(event.deltaY < 0 ? 10 : -10);
}

function HeaderRow({ label, value }: { label: string; value: string }) {
  return (
    <>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </>
  );
}

function SourceView({ source }: { source: string }) {
  return (
    <article class="document source-view">
      <pre>{source}</pre>
    </article>
  );
}

function ErrorState({ message }: { message: string }) {
  return (
    <section class="empty-state error-state">
      <h1>Could not open file</h1>
      <p>{message}</p>
    </section>
  );
}

function InfoDialog({ document, onClose }: { document: DocumentPayload; onClose: () => void }) {
  const viewer = viewerRegistry[document.kind];
  return (
    <div class="modal-backdrop" role="presentation" onClick={onClose}>
      <section class="info-dialog" role="dialog" aria-modal="true" aria-label="File information" onClick={(event) => event.stopPropagation()}>
        <header class="info-dialog-header">
          <div>
            <h1>{document.fileName}</h1>
            <span>{viewer.label}</span>
          </div>
          <button type="button" class="icon-button" aria-label="Close" onClick={onClose}>
            ×
          </button>
        </header>

        <div class="info-dialog-body">
          <InfoSection title="General" entries={document.info.general} />
          {document.info.sections.map((section) => (
            <InfoSection key={section.title} title={section.title} entries={section.entries} />
          ))}
        </div>
      </section>
    </div>
  );
}

function InfoSection({ title, entries }: { title: string; entries: InfoEntry[] }) {
  if (entries.length === 0) {
    return null;
  }

  return (
    <section class="info-section">
      <h2>{title}</h2>
      <dl>
        {entries.map((entry) => (
          <InfoRow key={`${entry.label}-${entry.value}`} entry={entry} />
        ))}
      </dl>
    </section>
  );
}

function InfoRow({ entry }: { entry: InfoEntry }) {
  return (
    <>
      <dt>{entry.label}</dt>
      <dd title={entry.value}>{formatInfoValue(entry)}</dd>
    </>
  );
}

function formatInfoValue(entry: InfoEntry) {
  if (entry.label === "Modified" && /^\d+$/.test(entry.value)) {
    return new Date(Number(entry.value) * 1000).toLocaleString();
  }

  return entry.value;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function errorToMessage(error: unknown) {
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return "Something went wrong.";
}
