// Données fictives de démonstration, jamais imposées par le modèle.
// typst compile --root . examples/courrier-personnel.typ output/pdf/courrier-personnel.pdf
// Ajouter --input controle=true pour la planche de contrôle.
#import "../templates/courrier-personnel.typ": courrier
#show: courrier.with(
  expediteur: "Camille Martin",
  adresse_expediteur: "18 rue des Tilleuls\n35000 Rennes",
  email: "camille.martin@example.org",
  telephone: "06 00 00 00 00",
  destinataire: "Service des inscriptions",
  adresse: "Maison des associations\n4 avenue de la République\n35000 RENNES",
  objet: "Demande d’attestation d’inscription",
  lieu: "Rennes",
  date: "21 octobre 2026",
  signature: "/tests/fixtures/signature.svg",
  controle: sys.inputs.at("controle", default: "false") == "true",
)

Madame, Monsieur,

Je suis inscrit à l’atelier de photographie organisé par la Maison des associations pour l’année 2026-2027.

Afin de compléter mon dossier de demande de remboursement auprès de mon employeur, je vous remercie de bien vouloir me transmettre une attestation mentionnant mon inscription et le montant des frais acquittés.

Vous pouvez me l’adresser par courriel ou par voie postale, aux coordonnées indiquées en tête de ce courrier.

Je vous prie d’agréer, Madame, Monsieur, mes salutations distinguées.
