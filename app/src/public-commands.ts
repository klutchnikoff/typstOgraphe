import type { PublicCommand, Registry } from './commands'
import { allowed, refused, availabilityLabel, type Availability } from './command-availability'

export function commandAvailability(command: PublicCommand): Availability {
  return (
    command.availability?.() || (command.enabled?.() === false ? refused('unavailable') : allowed)
  )
}
export function runCommand(command: PublicCommand): boolean {
  if (!commandAvailability(command).enabled) return false
  command.run()
  return true
}
export function refreshCommandButton(button: HTMLButtonElement, command: PublicCommand) {
  const value = commandAvailability(command)
  button.disabled = !value.enabled
  button.setAttribute('aria-disabled', String(!value.enabled))
  button.title = [command.label, availabilityLabel(value)].filter(Boolean).join(' — ')
}
/** Every registration is guarded, including direct calls to .run() from future
 * triggers. The shell adds document/modal policy once, after registering its tools.
 */
export class CommandRegistry extends Map<string, PublicCommand> {
  policy: (command: PublicCommand) => Availability = () => allowed
  override set(id: string, command: PublicCommand) {
    const own = () => commandAvailability(command)
    const availability = () => {
      const global = this.policy(command)
      return global.enabled ? own() : global
    }
    return super.set(id, {
      ...command,
      availability,
      enabled: () => availability().enabled,
      run: () => {
        if (availability().enabled) command.run()
      },
    })
  }
}
export function setCommandPolicy(registry: Registry, policy: CommandRegistry['policy']) {
  if (registry instanceof CommandRegistry) registry.policy = policy
}
