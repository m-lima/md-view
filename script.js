function toggleMode() {
  if (document.documentElement.getAttribute('data-theme') === 'dark') {
    document.documentElement.setAttribute('data-theme', 'light');
  } else {
    document.documentElement.setAttribute('data-theme', 'dark');
  }
}

function updateMarkdown(html) {
  document.getElementById('md').innerHTML = html;
}

function updatePadding(value) {
  const padderValue = document.getElementById('padder-value');
  if (!!padderValue) {
    padderValue.textContent = `${value}%`
  }

  const padder = document.getElementById('padder');
  if (!!padder) {
    padder.style['padding-right'] = `${value}%`
    padder.style['padding-left'] = `${value}%`
  }
}
