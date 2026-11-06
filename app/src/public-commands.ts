import type { PublicCommand, Registry } from './commands'
import { allowed, refused, availabilityLabel, type Availability } from './command-availability'

/** Type support is distinct from temporary contextual availability. */
export function commandVisible(command: PublicCommand) {
  return command.visible?.() !== false
}
export function commandAvailability(command: PublicCommand): Availability {
  if (!commandVisible(command)) return refused('unsupported_document_type')
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
  const shortcut = command.shortcut?.replace(
    'Mod',
    navigator.platform.startsWith('Mac') ? 'Cmd' : 'Ctrl',
  )
  const label = shortcut ? `${command.label} (${shortcut})` : command.label
  button.title = [label, availabilityLabel(value)].filter(Boolean).join(' — ')
}
/** Every registration is guarded, including direct calls to .run() from future
 * triggers. The shell adds document/modal policy once, after registering its tools.
 */
export class CommandRegistry extends Map<string, PublicCommand> {
  constructor(private readonly supports: (command: PublicCommand) => boolean = () => true) {
    super()
  }
  policy: (command: PublicCommand) => Availability = () => allowed
  override set(id: string, command: PublicCommand) {
    const own = () => commandAvailability(command)
    const visible = () => this.supports(command) && commandVisible(command)
    const availability = () => {
      if (!visible()) return refused('unsupported_document_type')
      const global = this.policy(command)
      return global.enabled ? own() : global
    }
    return super.set(id, {
      ...command,
      visible,
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
