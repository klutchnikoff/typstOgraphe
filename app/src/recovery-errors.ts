import { tr } from './i18n'
/** Keep recovery failures readable; technical details remain available for diagnosis. */
export function recoveryError(reason: unknown) {
  const raw = String(reason)
  const messages: Record<string, string> = {
    recovery_invalid_file: 'Cette copie de secours est invalide.',
    recovery_invalid_directory: 'Le dossier de récupération n’est pas disponible.',
    recovery_invalid_reference: 'Cette copie de secours n’est plus disponible.',
    recovery_live_copy: 'Cette copie appartient à une fenêtre encore ouverte.',
    recovery_too_large: 'Cette copie de secours est trop volumineuse.',
    recovery_invalid_item: 'Une saisie récupérée ne peut pas être interprétée.',
    recovery_transition_busy: 'Une autre opération est déjà en cours. Réessayez.',
    recovery_transition_required:
      'Les autres fenêtres doivent conserver leur travail avant de continuer.',
    recovery_transition_not_ready: 'La conservation des autres fenêtres n’est pas terminée.',
    recovery_transition_not_conserved:
      'La copie de secours d’une fenêtre n’a pas pu être confirmée.',
    recovery_transition_obsolete: 'Le document a changé. Réessayez.',
    recovery_unknown_item: 'Cette saisie n’est plus disponible.',
  }
  const found = Object.keys(messages).find((key) => raw.includes(key))
  return found ? tr(messages[found]) : raw
}
