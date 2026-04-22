// Phase-1a placeholder board. Real story-list + drill-in is a separate
// follow-up per the handoff; for now just keeps the right-column
// geometry honest so chat doesn't shift when the board lands.
function BoardRegion(): React.JSX.Element {
  return (
    <aside className="board" data-test="board-region">
      <div className="board-header">board</div>
      <div className="board-body">
        <div className="board-placeholder">board coming soon</div>
      </div>
    </aside>
  )
}

export default BoardRegion
