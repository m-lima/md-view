function updateMarkdown(html) {
  document.getElementById('md').innerHTML = html;
  external.invoke('updated');
}
