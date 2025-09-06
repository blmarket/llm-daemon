import test from 'ava'

import { spawnDaemon } from '../index'

test('sync function from native code', (t) => {
  spawnDaemon('', []);
  t.pass();
})
