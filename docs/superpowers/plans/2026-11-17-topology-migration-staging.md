# Controlled migration staging and recovery

Implement the approved shared adoption proposal as a recoverable, local transaction.
Do not activate the service or write shaping output.

1. Seal each proposal against registry edits. Keep its evidence deadline.

2. Reuse store validation to calculate the exact next registry document.

3. Save a bounded private journal before installation. Require an absent source and runtime owner for this first-adoption path.

4. Use an unsupported writer marker to block earlier runtime writers during installation. Current writers and normal edits also reject a pending journal.

5. Install the exact source. Commit the exact registry. Check both before removal of the marker and journal.

6. Resume only recognized base or target states. Reject unknown files without replacement. Allow cancellation only before source installation.

7. Test restarts after each durable boundary, conflicts, unsafe files, expiry, and repeated recovery. Run the full suite, format, and Clippy.

Recovery moves forward after source installation. A journal contains registry policy and protected static endpoints only.
Existing source replacement and migration of an active topology owner are outside this first-adoption API.
Mixed-version registry editing is not supported. Earlier runtime writers reject the temporary owner marker.

## Verification

The final full suite passed: 1,181 tests, zero failures, and 13 ignored tests.
All 445 active topology tests passed. The new transaction has 14 active tests.
Format, Clippy with warnings denied, whitespace, and installed-library scratch compilation passed.
The code has author review only. No live apply or production benchmark occurred.

The library APIs are complete. Enrollment and service-start integration remain separate work.
