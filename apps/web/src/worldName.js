// A World's name: the project name, unless it's the legacy "Unnamed Project"
// placeholder, in which case fall back to the scenario it was built to test.
export const worldName = (p) => {
  const n = (p?.name || '').trim()
  if (n && n !== 'Unnamed Project') return n
  const req = (p?.simulation_requirement || '').trim()
  if (req) return req.length > 70 ? req.slice(0, 70) + '…' : req
  return 'Untitled world'
}
