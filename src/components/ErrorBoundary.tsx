import { Component, type ReactNode } from "react";

/** Last-resort recovery screen instead of a blank window. */
export class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className="flex h-full items-center justify-center p-8">
        <div className="max-w-md rounded-2xl border border-line bg-panel p-6 text-center">
          <p className="text-lg font-semibold">Something went wrong</p>
          <p className="mt-2 text-sm text-muted">The screen hit an unexpected error. Your chats and models are safe.</p>
          <pre className="mt-3 max-h-32 overflow-auto rounded-lg bg-bg/70 p-2 text-left text-xs text-bad">{this.state.error.message}</pre>
          <button onClick={() => location.reload()} className="mt-4 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-bg hover:brightness-110">
            Reload
          </button>
        </div>
      </div>
    );
  }
}
