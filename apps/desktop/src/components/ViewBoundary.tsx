import { Component, type ReactNode } from "react";

export class ViewBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <section className="panel device-empty" role="alert">
        <h2>This view could not be displayed</h2>
        <p>Try again or open another section.</p>
        <button className="secondary-button" onClick={() => this.setState({ failed: false })}>
          Retry view
        </button>
      </section>
    );
  }
}
