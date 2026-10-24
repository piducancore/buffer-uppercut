# ADR 0017: Press-order performance chain

- Status: accepted for listening test
- Date: 2026-10-23

Continuous stages run in the order their aggregate rising holds are observed.
A release removes the stage; repress appends it. Requests first observed within
one audio block tie-break by pad number. Sub-block event ordering remains deferred.
The six-stage cap and oldest-admission suspension policy remain unchanged.
Restoration resumes a stage at its original press position.

Unheld buffer histories record final output, preparing lookback for a stage
appended at the chain end. Held/suspended histories record at their press
position. Existing captures are retained when upstream membership changes.
A fixed stack array is sorted once per block without allocation. Telemetry carries
chain position so the editor displays the same order as the DSP.

This supersedes the fixed-slot routing and history positions in ADR 0004 and
the ascending-pad display order in ADR 0016. Parameters and preset formats do
not change. CPU and listening acceptance remain open.
