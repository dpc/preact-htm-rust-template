import { html, render, useState, createContext, useContext } from 'https://unpkg.com/htm/preact/standalone.module.js'

const MyButton = () => {
  const [clickCount, setClickCount] = useState(0)

  const handleClick = () => {
    setClickCount(clickCount + 1)
  }

  return html`
    <button onClick=${handleClick}>
      Clicks: ${clickCount}
    </button>
  `;
}

const CpusCtx = createContext([])

function Cpus(props) {
  const cpus = useContext(CpusCtx)
  
  return html`
    <div>
      ${cpus.map((cpu) => {
        return html`<div class="bar">
          <div class="bar-inner" style="width: ${cpu}%"></div>
          <label>${cpu.toFixed(2)}%</label>
        </div>`;
      })
    }
    </div>
  `;
}


function App(props) {
  return html`
    <div>
     <${CpusCtx.Provider} value=${props.cpus}>
     <${Cpus}/>
     </${CpusCtx.Provider}>
     <${MyButton}/>
    </div>
  `;
}

let url = new URL("/events", window.location.href);

url.protocol = url.protocol.replace("http", "ws");

let ws = new WebSocket(url.href);
ws.onmessage = (ev) => {
  let event = JSON.parse(ev.data);
  render(html`<${App} cpus=${event.cpus}></${App}>`, document.body);
};
