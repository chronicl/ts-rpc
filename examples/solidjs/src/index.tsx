/* @refresh reload */
import { createSignal } from 'solid-js';
import { render } from 'solid-js/web';
import { login } from '../../api';

function App() {
  let [email, setEmail] = createSignal('');

  // You can access the types associated with the login endpoint
  // by using the login namespace, i.e. login.Password is the password type.
  login('email', { password: 'password' }).then(e => setEmail(e));

  return <div>Email: {email()}</div>;
}

render(() => <App />, document.body);
