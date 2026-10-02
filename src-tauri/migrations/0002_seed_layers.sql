-- Campus — semente de camadas (spec 01, Fase 1.9 / D4).
-- As 6 camadas do Calendário, na ordem de empilhamento da grade.
-- O id é estável e legível porque é o mesmo id de `LayerId` em
-- `src/features/calendar/domain/layers.ts` — não é um UUID.
-- A camada `google` nasce invisível e não editável: item importado é leitura.

INSERT INTO layer (id, title, tone, icon, visible, position) VALUES
  ('faculdade',    'Faculdade',        '2', 'graduation-cap',  1, 1),
  ('estudos',      'Estudos',          '5', 'book-open',       1, 2),
  ('tcc',          'TCC',              '4', 'file-text',       1, 3),
  ('estagio',      'Estágio',          '1', 'building',        1, 4),
  ('conhecimento', 'Conhecimento',     '3', 'library',         1, 5),
  ('google',       'Google Calendar', 'neutral', 'calendar',   0, 6);
