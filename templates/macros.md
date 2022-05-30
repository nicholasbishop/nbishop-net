{%- macro toc(contents) %}
{%- for item in contents -%}
* {{ item.date }} - [{{ item.title }}]({{ item.target }})
{% endfor %}
{% endmacro -%}

{% macro photo(name, text) %}
<a href="images-{{name}}.jpg"><img class="photo" src="images-{{name}}-thumb.jpg" title="{{text}}"></img></a>
{% endmacro %}
