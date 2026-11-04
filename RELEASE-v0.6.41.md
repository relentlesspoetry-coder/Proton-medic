# Proton Medic v0.6.41

- Show a confirmation window when Windows games or installers are active during an update. Each session is listed by name and type.
- Choose **Keep running** to cancel the update, or **Stop and update** to request shutdown of the listed sessions. Unsaved game progress or an interrupted installation may be lost.
- Wait for supervised processes and their completion records before installing. Processes that do not exit remain protected by the update lock; they are never force-killed.
- Sessions started by older versions must be closed manually. Downloads, repairs, and queued logging setup must still finish before updating.
